## Verdict

No — the codebase as captured in `codebase_context.md` does **not** fully implement Branch 7 + `spec.md` yet, because the **formula parser is still failing Branch 7 parser tests**. In `cycle_summary.txt`, `f7_formula_parser_tests` fails on:

* `Table1[Column1]` with a “trailing characters” parse error
* `#DIV/0!` with a “trailing characters” parse error 

This directly violates `spec.md`’s “Definition of done” item (1): `core/src/formula.rs` exists with AST + parser **and tests pass**. 

The rest of Branch 7 (7.3/7.4) looks implemented and wired correctly (formula diff caching, feature flag gating, adding `formula_diff` onto `CellEdited`, and engine propagation), but Branch 7 is not “done” until the parser is fixed and the test suite is green.

---

## What is already implemented correctly

These parts appear to match `spec.md` and the Branch 7 plan:

* `FormulaDiffResult` exists and includes variants like `FormattingOnly`, `Filled`, `SemanticChange`, etc. 
* `DiffOp::CellEdited` includes `formula_diff` (with JSON shape tests updated to include the field).
* `diff_cell_formulas_ids(...)` exists, is cached by `StringId`, and has a fast path when `enable_formula_semantic_diff == false` (no parsing).
* Engine wiring computes shifts and passes them into formula diff computation (via helpers like `compute_formula_diff` / `emit_cell_edit`).
* Branch 7 integration tests exist and are passing in the run captured by `cycle_summary.txt` (the suite stops because the parser tests fail). 

---

## What is missing or incorrect

### 1) Excel error literals are parsed incorrectly (`#DIV/0!`, `#NAME?`, etc.)

Current `parse_error` never consumes the leading `#`, and only allows a tiny set of characters, so it stops immediately on `#` and leaves the rest as trailing input. That matches the observed failure for `#DIV/0!` (pos 1 trailing characters).

### 2) Structured/table references are not supported (`Table1[Column1]`)

The parser accepts `Table1` as an identifier, but it does not consume the subsequent `[Column1]`, leaving a trailing `[` at pos 6 — exactly what the failing test reports.

`spec.md` explicitly calls for parsing a bracket blob after an identifier to handle structured references.

### 3) Canonicalization tests are still missing (per Branch 7 deliverables)

Branch 7 planning explicitly calls out adding canonicalization tests for commutative operators/functions.
I did not see dedicated tests that assert commutative reordering behavior (e.g., `A1+B1` canonicalizes equal to `B1+A1`) in the provided context.

---

## Implementation plan to fix all omissions/errors

### Step 1 — Fix `parse_error` to actually consume error literals

**File:** `core/src/formula.rs`

#### Code to replace (current `parse_error`)

```rust
fn parse_error(&mut self) -> Result<FormulaExpr, FormulaParseError> {
    let start = self.pos;
    while let Some(b) = self.peek() {
        self.pos += 1;
        if b.is_ascii_alphabetic() || b == b'/' || b == b'0' || b == b'!' {
            continue;
        } else {
            break;
        }
    }
    let txt = std::str::from_utf8(&self.s[start..self.pos]).unwrap();
    let err = match txt.to_ascii_uppercase().as_str() {
        "#NULL!" => ExcelError::Null,
        "#DIV/0!" => ExcelError::Div0,
        "#VALUE!" => ExcelError::Value,
        "#REF!" => ExcelError::Ref,
        "#NAME?" => ExcelError::Name,
        "#NUM!" => ExcelError::Num,
        "#N/A" => ExcelError::NA,
        "#SPILL!" => ExcelError::Spill,
        "#CALC!" => ExcelError::Calc,
        "#GETTING_DATA" => ExcelError::GettingData,
        other => ExcelError::Unknown(other.to_string()),
    };
    Ok(FormulaExpr::Error(err))
}
```

#### New code to replace it with

```rust
fn parse_error(&mut self) -> Result<FormulaExpr, FormulaParseError> {
    let start = self.pos;

    if self.bump() != Some(b'#') {
        return Err(self.err("expected '#'"));
    }

    while let Some(b) = self.peek() {
        if b.is_ascii_alphanumeric() || matches!(b, b'/' | b'!' | b'?' | b'_') {
            self.pos += 1;
        } else {
            break;
        }
    }

    let txt = std::str::from_utf8(&self.s[start..self.pos])
        .map_err(|_| self.err("invalid utf-8 in error literal"))?
        .to_string();

    let upper = txt.to_ascii_uppercase();
    let err = match upper.as_str() {
        "#NULL!" => ExcelError::Null,
        "#DIV/0!" => ExcelError::Div0,
        "#VALUE!" => ExcelError::Value,
        "#REF!" => ExcelError::Ref,
        "#NAME?" => ExcelError::Name,
        "#NUM!" => ExcelError::Num,
        "#N/A" => ExcelError::NA,
        "#SPILL!" => ExcelError::Spill,
        "#CALC!" => ExcelError::Calc,
        "#GETTING_DATA" => ExcelError::GettingData,
        _ => ExcelError::Unknown(txt),
    };

    Ok(FormulaExpr::Error(err))
}
```

This change is enough to resolve the `#DIV/0!` failure shown in `cycle_summary.txt`. 

---

### Step 2 — Add a structured-reference bracket parser helper

**File:** `core/src/formula.rs`
**Action:** Add this method to `impl Parser` (near other `parse_*` helpers).

```rust
fn parse_bracket_blob(&mut self) -> Result<String, FormulaParseError> {
    self.skip_ws();
    if self.peek() != Some(b'[') {
        return Err(self.err("expected '['"));
    }

    let start = self.pos;
    let mut depth: i32 = 0;

    while let Some(b) = self.bump() {
        match b {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
    }

    if depth != 0 {
        return Err(self.err("unterminated structured reference"));
    }

    let txt = std::str::from_utf8(&self.s[start..self.pos])
        .map_err(|_| self.err("invalid utf-8 in structured reference"))?
        .to_string();

    Ok(txt)
}
```

This matches the approach in `spec.md` (parse bracket blob, allow nesting) and is the required primitive to support `Table1[Column1]`. 

---

### Step 3 — Extend identifier parsing in `parse_ref_or_name` to consume structured references

**File:** `core/src/formula.rs`

You can do this as a minimal, localized edit: replace the tail section that currently parses identifier/boolean/function call/named ref, so it checks for `[` after the identifier before deciding it’s a plain name.

#### Code to replace (current tail after `let ident = self.parse_identifier()?;`)

```rust
let ident = self.parse_identifier()?;
if ident.eq_ignore_ascii_case("TRUE") {
    return Ok(FormulaExpr::Boolean(true));
}
if ident.eq_ignore_ascii_case("FALSE") {
    return Ok(FormulaExpr::Boolean(false));
}

self.skip_ws();
if self.peek() == Some(b'(') {
    self.bump();
    let mut args = Vec::new();
    loop {
        self.skip_ws();
        if self.peek() == Some(b')') {
            self.bump();
            break;
        }
        let arg = self.parse_expr(0)?;
        args.push(arg);
        self.skip_ws();
        match self.peek() {
            Some(b',') | Some(b';') => {
                self.bump();
            }
            Some(b')') => {
                self.bump();
                break;
            }
            _ => return Err(self.err("expected ',' or ')'")),
        }
    }
    Ok(FormulaExpr::FunctionCall { name: ident, args })
} else {
    Ok(FormulaExpr::NamedRef(ident))
}
```

#### New code to replace it with

```rust
let ident = self.parse_identifier()?;
self.skip_ws();

if sheet.is_none()
    && ident.eq_ignore_ascii_case("TRUE")
    && self.peek() != Some(b'(')
    && self.peek() != Some(b'[')
{
    return Ok(FormulaExpr::Boolean(true));
}
if sheet.is_none()
    && ident.eq_ignore_ascii_case("FALSE")
    && self.peek() != Some(b'(')
    && self.peek() != Some(b'[')
{
    return Ok(FormulaExpr::Boolean(false));
}

if self.peek() == Some(b'[') {
    let structured = self.parse_bracket_blob()?;
    let full = match sheet {
        Some(s) => format!("{}!{}{}", s, ident, structured),
        None => format!("{}{}", ident, structured),
    };
    return Ok(FormulaExpr::NamedRef(full));
}

if self.peek() == Some(b'(') {
    self.bump();
    let mut args = Vec::new();
    loop {
        self.skip_ws();
        if self.peek() == Some(b')') {
            self.bump();
            break;
        }
        let arg = self.parse_expr(0)?;
        args.push(arg);
        self.skip_ws();
        match self.peek() {
            Some(b',' | b';') => {
                self.bump();
            }
            Some(b')') => {
                self.bump();
                break;
            }
            _ => return Err(self.err("expected ',' or ')'")),
        }
    }

    let name = match sheet {
        Some(s) => format!("{}!{}", s, ident),
        None => ident,
    };

    return Ok(FormulaExpr::FunctionCall { name, args });
}

let name = match sheet {
    Some(s) => format!("{}!{}", s, ident),
    None => ident,
};

Ok(FormulaExpr::NamedRef(name))
```

This is the change that eliminates the `Table1[Column1]` “trailing characters” error observed in `cycle_summary.txt`. 

It also follows `spec.md`’s structured-ref expectation (“if `[` after name, parse bracket blob and return NamedRef”). 

---

### Step 4 — Align `parse_string` with `spec.md` (escaped quotes)

This is not currently the cause of the failing tests, but it is part of the parser robustness described in `spec.md`, and it’s a cheap correctness win.

**File:** `core/src/formula.rs`

#### Code to replace (current `parse_string`)

```rust
fn parse_string(&mut self) -> Result<FormulaExpr, FormulaParseError> {
    self.bump();
    let start = self.pos;
    while let Some(b) = self.peek() {
        self.pos += 1;
        if b == b'"' {
            let s = std::str::from_utf8(&self.s[start..self.pos - 1])
                .map_err(|_| self.err("invalid utf-8 in string"))?
                .to_string();
            return Ok(FormulaExpr::Text(s));
        }
    }
    Err(self.err("unterminated string"))
}
```

#### New code to replace it with

```rust
fn parse_string(&mut self) -> Result<FormulaExpr, FormulaParseError> {
    if self.bump() != Some(b'"') {
        return Err(self.err("expected '\"'"));
    }

    let mut out = Vec::new();
    loop {
        match self.bump() {
            Some(b'"') => {
                if self.peek() == Some(b'"') {
                    self.bump();
                    out.push(b'"');
                    continue;
                }
                break;
            }
            Some(b) => out.push(b),
            None => return Err(self.err("unterminated string literal")),
        }
    }

    Ok(FormulaExpr::Text(String::from_utf8(out).unwrap_or_default()))
}
```

This matches the intent in `spec.md` (handle doubled quotes). 

---

### Step 5 — Add canonicalization tests for commutative normalization

This is required by the Branch 7 plan + `spec.md` definition-of-done checklist (“canonicalization … is tested”).

**Create new file:** `core/tests/f7_formula_canonicalization_tests.rs`

```rust
use excel_diff::parse_formula;

#[test]
fn canonicalizes_commutative_binary_ops() {
    let a = parse_formula("A1+B1").unwrap().canonicalize();
    let b = parse_formula("B1+A1").unwrap().canonicalize();
    assert_eq!(a, b);

    let a = parse_formula("A1*B1").unwrap().canonicalize();
    let b = parse_formula("B1*A1").unwrap().canonicalize();
    assert_eq!(a, b);
}

#[test]
fn canonicalizes_commutative_functions_by_sorting_args() {
    let a = parse_formula("SUM(A1,B1)").unwrap().canonicalize();
    let b = parse_formula("SUM(B1,A1)").unwrap().canonicalize();
    assert_eq!(a, b);

    let a = parse_formula("AND(TRUE,FALSE)").unwrap().canonicalize();
    let b = parse_formula("AND(FALSE,TRUE)").unwrap().canonicalize();
    assert_eq!(a, b);
}

#[test]
fn does_not_canonicalize_non_commutative_ops() {
    let a = parse_formula("A1-B1").unwrap().canonicalize();
    let b = parse_formula("B1-A1").unwrap().canonicalize();
    assert_ne!(a, b);
}

#[test]
fn canonicalizes_range_endpoints() {
    let a = parse_formula("B2:A1").unwrap().canonicalize();
    let b = parse_formula("A1:B2").unwrap().canonicalize();
    assert_eq!(a, b);
}

#[test]
fn structured_refs_parse_and_canonicalize() {
    let a = parse_formula("Table1[Column1]").unwrap().canonicalize();
    let b = parse_formula("TABLE1[COLUMN1]").unwrap().canonicalize();
    assert_eq!(a, b);
}
```

---

### Step 6 — Re-run the test suite and verify Branch 7 “done” checklist

After Steps 1–5:

* `core/tests/f7_formula_parser_tests.rs` should pass (fixes `#DIV/0!` and `Table1[Column1]`). 
* Integration tests already passing (`f7_formula_diff_integration_tests`) should remain green. 
* You’ll now satisfy the “Definition of done” items that are currently blocked (parser tests + canonicalization tests).

---

## Optional (but recommended) follow-up from `spec.md`: remove `format!("{:?}", ...)` sort keys

Your current canonicalization uses `format!("{:?}", ...)` as a sort key for commutative reordering, which `spec.md` flags as correct-but-slower and suggests replacing with a lightweight canonical hash.
I would do this only after the correctness fixes above are merged and the parser suite is green.

---

If you want, I can also produce a short “verification matrix” that maps each Branch 7 checklist item from `spec.md` to the exact file(s)/function(s) in `codebase_context.md` that implement it, and indicate which ones are now satisfied after the patches above.
