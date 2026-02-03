### What the benchmarks are telling you

Your slowest numbers are coming almost entirely from **workbook parsing**, not from the diff itself.

* The **e2e suite** totals **31,091 ms**, and in every case **parse_time_ms ≈ total_time_ms**. 
* The standout is **`e2e_p5_identical`**: **21,292 ms total**, with **21,278 ms parse** and only **14 ms diff**. It also hits **~762 MB peak memory**, with **~480 MB in the string pool** and **~240 MB in grid storage**. 

That means: if you want the biggest benchmark win, focus on the **“open + parse .xlsx”** pipeline (worksheet XML parsing, shared strings parsing, and address parsing), not the diff engine.

This lines up with how the e2e harness is written: it opens two fixtures via `WorkbookPackage::open_with_limits(...)` and then runs `diff_streaming(...)`, and the metrics explicitly check that `parse_time_ms` is the bulk of `total_time_ms`. 

---

## The main bottleneck in the code: buffering every parsed cell

The worksheet XML parser currently does this:

1. Scan the sheet XML.
2. For every `<c>` cell, parse it into a `ParsedCell`.
3. Push all `ParsedCell`s into a `Vec`.
4. After reaching EOF, decide dense-vs-sparse, allocate a grid, and **loop the vector again** inserting cells.

Concretely, you have:

* `let mut parsed_cells: Vec<ParsedCell> = Vec::new();` and `parsed_cells.push(cell);` during parsing 
* Then `build_grid(...)` allocates either dense or sparse and loops `for parsed in cells { grid.insert_cell(...) }` 

For sheets like `e2e_p5_identical` that effectively represent **millions of populated cells** (`cells_compared` is 5,000,100), this “buffer then replay” approach creates a large extra memory footprint and a lot of extra work. 

**A practical way to improve the benchmarks:** change worksheet parsing to avoid materializing a full `Vec<ParsedCell>` for large sheets, and instead build the `Grid` as you parse.

### Why this should move the benchmark needle

Because the e2e suite is dominated by parsing time (e.g., 21.278 s / 21.292 s for `p5`), any reduction in worksheet parse overhead translates almost 1:1 into `total_time_ms`. 

---

## A second hot spot: A1 address parsing does per-character Unicode iteration

Every parsed cell runs:

```rust
let (row, col) = address_to_index(address_raw.as_ref()) ...
```

and `address_to_index` is currently implemented using `for ch in a1.chars()`.

For large dense worksheets, this is done **millions of times**, and `chars()` means UTF-8 decoding + branching you do not need (cell addresses are ASCII). Switching to a byte-based parser is a low-risk micro-optimization that can stack with the “don’t buffer ParsedCell” improvement.

---

# Recommended “means of improvement” (highest ROI)

## 1) Stream worksheet XML directly into the grid (avoid `Vec<ParsedCell>` for big sheets)

**Goal:** Don’t store every cell twice (once in `Vec`, once in `Grid`). Parse → insert.

A workable strategy that keeps behavior correct:

* Keep reading `<dimension ref="...">` early (you already do) to get a reliable `(nrows, ncols)` when it exists.
* Once you have dimensions, start building the grid during parsing rather than after parsing.

Two implementation approaches:

**Approach A (simple, safe):**

* Continue buffering at first, but only up to a fixed cap (e.g., N cells).
* After the cap is reached *and* dimensions are known, allocate the target grid and drain buffered cells into it, then insert directly as you continue parsing.

This limits peak memory and removes the “second full pass over millions of cells” for the bulk of a dense sheet.

**Approach B (best long-term):**

* Parse `<row r="...">` and build row-by-row rather than cell-by-cell buffering.
* Use early sampling of the first K rows to determine density (dense sheets look dense immediately), so you can confidently allocate dense storage early without waiting until you’ve seen 40% of all cells.

Either approach will reduce:

* parse-time overhead (less buffering, less copying/moving),
* peak memory (no massive `Vec<ParsedCell>` sitting alongside the allocated dense grid).

This targets exactly the dominant cost in your benchmarks.

---

## 2) Quick win patch: rewrite `address_to_index` to parse ASCII bytes

This is a small change with broad impact because it is used by:

* cell parsing (`parse_cell`) for every `<c r="A1">` 
* dimension parsing (`dimension_from_ref`) for sheet size inference 

Below is a drop-in replacement that keeps the same semantics (rejects malformed addresses; uses checked arithmetic).

### Code to replace (current `address_to_index`)

```rust
pub fn address_to_index(a1: &str) -> Option<(u32, u32)> {
    if a1.is_empty() {
        return None;
    }

    let mut col: u32 = 0;
    let mut row: u32 = 0;
    let mut saw_letter = false;
    let mut saw_digit = false;

    for ch in a1.chars() {
        if ch.is_ascii_alphabetic() {
            saw_letter = true;
            if saw_digit {
                // Letters after digits are not allowed.
                return None;
            }
            let upper = ch.to_ascii_uppercase() as u8;
            if !upper.is_ascii_uppercase() {
                return None;
            }
            col = col
                .checked_mul(26)?
                .checked_add((upper - b'A' + 1) as u32)?;
        } else if ch.is_ascii_digit() {
            saw_digit = true;
            row = row.checked_mul(10)?.checked_add((ch as u8 - b'0') as u32)?;
        } else {
            return None;
        }
    }

    if !saw_letter || !saw_digit || row == 0 || col == 0 {
        return None;
    }

    Some((row - 1, col - 1))
}
```

### New code to replace it with (byte-based)

```rust
pub fn address_to_index(a1: &str) -> Option<(u32, u32)> {
    let bytes = a1.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let mut i: usize = 0;
    let mut col: u32 = 0;

    // Parse column letters (A..Z, a..z)
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_alphabetic() {
            let upper = b.to_ascii_uppercase();
            col = col
                .checked_mul(26)?
                .checked_add((upper - b'A' + 1) as u32)?;
            i += 1;
        } else {
            break;
        }
    }

    if col == 0 {
        return None;
    }

    // Must have at least one digit for the row.
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        return None;
    }

    let mut row: u32 = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_digit() {
            row = row
                .checked_mul(10)?
                .checked_add((b - b'0') as u32)?;
            i += 1;
        } else {
            // No trailing non-digits allowed.
            return None;
        }
    }

    if row == 0 {
        return None;
    }

    Some((row - 1, col - 1))
}
```
