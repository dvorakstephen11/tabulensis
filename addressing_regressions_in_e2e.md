Yes — the pattern in your results lines up with two “hot-path” costs that disproportionately hit those three cases:

* **`e2e_p2_noise` + `e2e_p3_repetitive`**: those sheets are much more **numeric- and/or shared-string-index heavy** than `p1_dense`. Right now the parser leans on `str::parse()` for both `usize` (shared string indexes) and `f64` (numbers). That’s convenient but relatively expensive at millions of cells.
* **`perf_50k_identical`**: the “identical grid” short-circuit still ends up doing a full cell equality scan, and your `CellValue::Number` equality always goes through `normalize_float_for_hash`, which is far more expensive than needed when values are *bit-identical* (the dominant case in identical comparisons).

Below are changes that should keep all the wins you already got, while cutting those regressions.

---

## 1) Speed up numeric + shared-string-index parsing (`core/src/grid_parser.rs`)

### Replace this `parse_cell` function

```rust
fn parse_cell(
    reader: &mut Reader<&[u8]>,
    xml: &[u8],
    start: BytesStart,
    shared_strings: &[StringId],
    pool: &mut StringPool,
    buf: &mut Vec<u8>,
) -> Result<ParsedCell, GridParseError> {
    let address_raw = get_attr_value(reader, xml, &start, b"r")?
        .ok_or_else(|| xml_msg_err(reader, xml, "cell missing address"))?;
    let (row, col) = address_to_index(address_raw.as_ref())
        .ok_or_else(|| GridParseError::InvalidAddress(address_raw.into_owned()))?;

    let cell_type = get_attr_value(reader, xml, &start, b"t")?;

    let mut value: Option<CellValue> = None;
    let mut formula: Option<StringId> = None;

    buf.clear();
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"v" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| xml_err(reader, xml, e))?;
                value = convert_value(
                    Some(text.as_ref()),
                    cell_type.as_deref(),
                    shared_strings,
                    pool,
                    reader,
                    xml,
                )?;
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"f" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| xml_err(reader, xml, e))?;
                let unescaped = quick_xml::escape::unescape(text.as_ref())
                    .map_err(|e| xml_msg_err(reader, xml, e.to_string()))?;
                formula = Some(pool.intern(unescaped.as_ref()));
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"is" => {
                let inline = read_inline_string(reader, xml)?;
                value = Some(CellValue::Text(pool.intern(&inline)));
            }
            Ok(Event::End(e)) if e.name().as_ref() == start.name().as_ref() => break,
            Ok(Event::Eof) => {
                return Err(xml_msg_err(reader, xml, "unexpected EOF inside cell"));
            }
            Err(e) => return Err(xml_err(reader, xml, e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(ParsedCell {
        row,
        col,
        value,
        formula,
    })
}
```

### With this version (single-pass attribute scan)

```rust
fn parse_cell(
    reader: &mut Reader<&[u8]>,
    xml: &[u8],
    start: BytesStart,
    shared_strings: &[StringId],
    pool: &mut StringPool,
    buf: &mut Vec<u8>,
) -> Result<ParsedCell, GridParseError> {
    let mut address_raw = None;
    let mut cell_type = None;

    for attr in start.attributes() {
        let attr = attr.map_err(|e| xml_msg_err(reader, xml, e.to_string()))?;
        match attr.key.as_ref() {
            b"r" => {
                address_raw =
                    Some(attr.unescape_value().map_err(|e| xml_err(reader, xml, e))?);
            }
            b"t" => {
                cell_type = Some(attr.unescape_value().map_err(|e| xml_err(reader, xml, e))?);
            }
            _ => {}
        }
    }

    let address_raw = address_raw.ok_or_else(|| xml_msg_err(reader, xml, "cell missing address"))?;
    let (row, col) = address_to_index(address_raw.as_ref())
        .ok_or_else(|| GridParseError::InvalidAddress(address_raw.into_owned()))?;

    let mut value: Option<CellValue> = None;
    let mut formula: Option<StringId> = None;

    buf.clear();
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"v" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| xml_err(reader, xml, e))?;
                value = convert_value(
                    Some(text.as_ref()),
                    cell_type.as_deref(),
                    shared_strings,
                    pool,
                    reader,
                    xml,
                )?;
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"f" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| xml_err(reader, xml, e))?;
                let unescaped = quick_xml::escape::unescape(text.as_ref())
                    .map_err(|e| xml_msg_err(reader, xml, e.to_string()))?;
                formula = Some(pool.intern(unescaped.as_ref()));
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"is" => {
                let inline = read_inline_string(reader, xml)?;
                value = Some(CellValue::Text(pool.intern(&inline)));
            }
            Ok(Event::End(e)) if e.name().as_ref() == start.name().as_ref() => break,
            Ok(Event::Eof) => {
                return Err(xml_msg_err(reader, xml, "unexpected EOF inside cell"));
            }
            Err(e) => return Err(xml_err(reader, xml, e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(ParsedCell {
        row,
        col,
        value,
        formula,
    })
}
```

---

### Replace this `convert_value` function

```rust
fn convert_value(
    value_text: Option<&str>,
    cell_type: Option<&str>,
    shared_strings: &[StringId],
    pool: &mut StringPool,
    reader: &Reader<&[u8]>,
    xml: &[u8],
) -> Result<Option<CellValue>, GridParseError> {
    let raw = match value_text {
        Some(t) => t,
        None => return Ok(None),
    };

    let trimmed = raw.trim();
    if raw.is_empty() || trimmed.is_empty() {
        return Ok(Some(CellValue::Text(pool.intern(""))));
    }

    match cell_type {
        Some("s") => {
            let idx = trimmed
                .parse::<usize>()
                .map_err(|e| xml_msg_err(reader, xml, e.to_string()))?;
            let text_id = *shared_strings
                .get(idx)
                .ok_or(GridParseError::SharedStringOutOfBounds(idx))?;
            Ok(Some(CellValue::Text(text_id)))
        }
        Some("b") => Ok(match trimmed {
            "1" => Some(CellValue::Bool(true)),
            "0" => Some(CellValue::Bool(false)),
            _ => None,
        }),
        Some("e") => Ok(Some(CellValue::Error(pool.intern(trimmed)))),
        Some("str") | Some("inlineStr") => Ok(Some(CellValue::Text(pool.intern(raw)))),
        _ => {
            if let Ok(n) = trimmed.parse::<f64>() {
                Ok(Some(CellValue::Number(n)))
            } else {
                Ok(Some(CellValue::Text(pool.intern(trimmed))))
            }
        }
    }
}
```

### With this version (fast-path decimal parsing for `usize` and integer `f64`)

```rust
fn convert_value(
    value_text: Option<&str>,
    cell_type: Option<&str>,
    shared_strings: &[StringId],
    pool: &mut StringPool,
    reader: &Reader<&[u8]>,
    xml: &[u8],
) -> Result<Option<CellValue>, GridParseError> {
    fn parse_usize_decimal(s: &str) -> Option<usize> {
        let bytes = s.as_bytes();
        if bytes.is_empty() {
            return None;
        }
        let mut n: usize = 0;
        for &b in bytes {
            if !b.is_ascii_digit() {
                return None;
            }
            let d = (b - b'0') as usize;
            n = n.checked_mul(10)?.checked_add(d)?;
        }
        Some(n)
    }

    fn parse_f64_fast(s: &str) -> Option<f64> {
        let bytes = s.as_bytes();
        if bytes.is_empty() {
            return None;
        }

        let mut i = 0usize;
        let mut neg = false;
        match bytes[0] {
            b'-' => {
                neg = true;
                i = 1;
            }
            b'+' => {
                i = 1;
            }
            _ => {}
        }

        if i >= bytes.len() {
            return None;
        }

        let mut int: u64 = 0;
        let mut saw_digit = false;

        while i < bytes.len() {
            let b = bytes[i];
            if b.is_ascii_digit() {
                saw_digit = true;
                let d = (b - b'0') as u64;
                if int > (u64::MAX - d) / 10 {
                    return s.parse::<f64>().ok();
                }
                int = int * 10 + d;
                i += 1;
                continue;
            }
            break;
        }

        if !saw_digit {
            return None;
        }

        if i == bytes.len() {
            let v = int as f64;
            return Some(if neg { -v } else { v });
        }

        match bytes[i] {
            b'.' | b'e' | b'E' => s.parse::<f64>().ok(),
            _ => None,
        }
    }

    let raw = match value_text {
        Some(t) => t,
        None => return Ok(None),
    };

    let trimmed = raw.trim();
    if raw.is_empty() || trimmed.is_empty() {
        return Ok(Some(CellValue::Text(pool.intern(""))));
    }

    match cell_type {
        Some("s") => {
            let idx = match parse_usize_decimal(trimmed) {
                Some(v) => v,
                None => trimmed
                    .parse::<usize>()
                    .map_err(|e| xml_msg_err(reader, xml, e.to_string()))?,
            };
            let text_id = *shared_strings
                .get(idx)
                .ok_or(GridParseError::SharedStringOutOfBounds(idx))?;
            Ok(Some(CellValue::Text(text_id)))
        }
        Some("b") => Ok(match trimmed {
            "1" => Some(CellValue::Bool(true)),
            "0" => Some(CellValue::Bool(false)),
            _ => None,
        }),
        Some("e") => Ok(Some(CellValue::Error(pool.intern(trimmed)))),
        Some("str") | Some("inlineStr") => Ok(Some(CellValue::Text(pool.intern(raw)))),
        _ => {
            if let Some(n) = parse_f64_fast(trimmed) {
                Ok(Some(CellValue::Number(n)))
            } else {
                Ok(Some(CellValue::Text(pool.intern(trimmed))))
            }
        }
    }
}
```

Why this helps your two regressing e2e cases: those are the ones where you’re doing millions of `parse::<f64>()` and/or `parse::<usize>()` calls. The fast path avoids the heavy general parser for the common “all digits” cases.

---

## 2) Avoid redundant signature invalidation during parsing (`core/src/workbook.rs`)

This is a quiet but meaningful cost when building large grids: `insert_cell` is called millions of times, and right now it writes `row_signatures = None` and `col_signatures = None` every time even though they are already `None` during parsing.

### Replace this `get_mut` + `insert_cell` block

```rust
pub fn get_mut(&mut self, row: u32, col: u32) -> Option<&mut CellContent> {
    self.row_signatures = None;
    self.col_signatures = None;
    self.cells.get_mut(row, col)
}

pub fn insert_cell(
    &mut self,
    row: u32,
    col: u32,
    value: Option<CellValue>,
    formula: Option<StringId>,
) {
    debug_assert!(
        row < self.nrows && col < self.ncols,
        "cell coordinates must lie within the grid bounds"
    );
    self.row_signatures = None;
    self.col_signatures = None;
    self.cells
        .insert(row, col, CellContent { value, formula });
    self.maybe_upgrade_to_dense();
}
```

### With this version

```rust
pub fn get_mut(&mut self, row: u32, col: u32) -> Option<&mut CellContent> {
    if self.row_signatures.is_some() {
        self.row_signatures = None;
    }
    if self.col_signatures.is_some() {
        self.col_signatures = None;
    }
    self.cells.get_mut(row, col)
}

pub fn insert_cell(
    &mut self,
    row: u32,
    col: u32,
    value: Option<CellValue>,
    formula: Option<StringId>,
) {
    debug_assert!(
        row < self.nrows && col < self.ncols,
        "cell coordinates must lie within the grid bounds"
    );
    if self.row_signatures.is_some() {
        self.row_signatures = None;
    }
    if self.col_signatures.is_some() {
        self.col_signatures = None;
    }
    self.cells
        .insert(row, col, CellContent { value, formula });
    self.maybe_upgrade_to_dense();
}
```

Why this helps `e2e_*`: `build_grid()` inserts every parsed cell. Cutting two big redundant stores per insert is a real win at 1M–2.5M cells.

---

## 3) Fix the `perf_50k_identical` regression with a fast numeric equality path (`core/src/workbook.rs`)

Right now numeric equality always computes `normalize_float_for_hash` even when the two floats are exactly the same bits (which is overwhelmingly the case in identical sheets).

### Replace this `impl PartialEq for CellValue`

```rust
impl PartialEq for CellValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CellValue::Blank, CellValue::Blank) => true,
            (CellValue::Number(a), CellValue::Number(b)) => {
                normalize_float_for_hash(*a) == normalize_float_for_hash(*b)
            }
            (CellValue::Text(a), CellValue::Text(b)) => a == b,
            (CellValue::Bool(a), CellValue::Bool(b)) => a == b,
            (CellValue::Error(a), CellValue::Error(b)) => a == b,
            _ => false,
        }
    }
}
```

### With this version

```rust
impl PartialEq for CellValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CellValue::Blank, CellValue::Blank) => true,
            (CellValue::Number(a), CellValue::Number(b)) => {
                if a.to_bits() == b.to_bits() {
                    return true;
                }
                if a.is_nan() && b.is_nan() {
                    return true;
                }
                if *a == 0.0 && *b == 0.0 {
                    return true;
                }
                normalize_float_for_hash(*a) == normalize_float_for_hash(*b)
            }
            (CellValue::Text(a), CellValue::Text(b)) => a == b,
            (CellValue::Bool(a), CellValue::Bool(b)) => a == b,
            (CellValue::Error(a), CellValue::Error(b)) => a == b,
            _ => false,
        }
    }
}
```

Why this helps `perf_50k_identical`: you’re doing ~5,000,000 numeric comparisons; this makes the common case (exactly equal numbers) extremely cheap while preserving your “normalized float” semantics for the hard cases.

---

## What I’d expect to happen to your three regressions

* **`e2e_p2_noise`**: should drop back at least to baseline, likely better, because it’s dominated by numeric parsing + grid insertion.
* **`e2e_p3_repetitive`**: should improve because it’s a lot of either numeric values or shared-string-index values; both are sped up.
* **`perf_50k_identical`**: should tighten substantially because the equality scan becomes much cheaper.

If you still see a stubborn regression on `e2e_p3_repetitive` after these, the next lever is making `read_inline_string` reuse a scratch buffer/string (to avoid per-cell allocations) — but I’d try the above first because it’s targeted and low-risk.
