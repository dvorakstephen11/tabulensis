Below is an implementation plan that maps directly onto **Branch 7** in `next_sprint_plan.md` (formula AST + parser, canonicalization, shift-equivalence, and integration into `CellEdited`) , tailored to how your current code stores formulas (`Option<StringId>` on cells/snapshots, without a leading `=`)  and how diffs are currently emitted via `DiffOp::cell_edited(...)` inside the engine .

I’m going to assume you want this to land as a clean, test-driven change that doesn’t perturb existing diff behavior unless the feature flag is enabled (as required by Branch 7) . Your config already has `enable_formula_semantic_diff` (default-off), so the main work is wiring + implementation .

---

## Key idea to anchor everything

A formula string is a poor representation for “meaning.” Two formulas can be meaningfully identical but textually different:

* `SUM(A1,B1)` vs `sum( A1 , B1 )` (case/whitespace only)
* `A1+B1` in `C1` vs `A2+B2` in `C2` when the row was inserted/fill-down happened (reference shift)

Branch 7’s approach is:

1. **Parse** formula text to an **AST** 
2. **Canonicalize** the AST so superficial differences disappear 
3. Detect **equivalence under shift** for fill patterns 
4. Feed that into `CellEdited` as a classification (`FormattingOnly`, `Filled`, `SemanticChange`, etc.) 

---

## Step 0 — Confirm current integration points

These are the “touchpoints” you’ll modify:

* **Formula storage:** `CellContent.formula: Option<StringId>` stored *without leading `=`* 
* **Cell edits emission:** engine functions call `DiffOp::cell_edited(...)` for changed cells 
* **Config flag exists:** `DiffConfig.enable_formula_semantic_diff` is already present 
* **Schema/ops:** `DiffOp::CellEdited` currently only carries `sheet, addr, from, to` 
* **There’s precedent:** `m_diff` already does semantic-vs-formatting classification for Power Query M 

This makes Branch 7 mostly “parallel” to the existing M semantic diff design.

---

## Step 1 — Branch 7.1: Add formula AST + parser

Branch 7.1 expects:

* `FormulaExpr` enum
* `CellReference`, `RangeReference`, `RowRef`, `ColRef`
* Parser supports common Excel syntax including structured refs, array constants, A1/R1C1 

### 1.1 Create a new module

Create: `core/src/formula.rs` and add it to `core/src/lib.rs`.

You’ll likely want the AST public (Branch 7 shows `pub` types) .

#### `core/src/lib.rs` module list

**Code to replace (snippet from current `lib.rs` module declarations):** 

```rust
mod hashing;
mod m_ast;
mod m_diff;
mod m_section;
mod output;
```

**New code to replace it with:**

```rust
mod hashing;
mod formula;
mod m_ast;
mod m_diff;
mod m_section;
mod output;
```

Then export the public API for formula parsing/types similarly to how M is exported.

---

### 1.2 `core/src/formula.rs` (new file)

This is a pragmatic AST that matches Branch 7’s shape but adds just enough to handle common real-world formulas safely.

Create **new file** `core/src/formula.rs`:

```rust
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum FormulaExpr {
    Number(f64),
    Text(String),
    Boolean(bool),
    Error(ExcelError),

    CellRef(CellReference),
    RangeRef(RangeReference),

    NamedRef(String),

    FunctionCall {
        name: String,
        args: Vec<FormulaExpr>,
    },

    UnaryOp {
        op: UnaryOperator,
        operand: Box<FormulaExpr>,
    },

    BinaryOp {
        op: BinaryOperator,
        left: Box<FormulaExpr>,
        right: Box<FormulaExpr>,
    },

    Array(Vec<Vec<FormulaExpr>>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExcelError {
    Null,
    Div0,
    Value,
    Ref,
    Name,
    Num,
    NA,
    Spill,
    Calc,
    GettingData,
    Unknown(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RowRef {
    Absolute(u32),
    Relative(u32),
    Offset(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColRef {
    Absolute(u32),
    Relative(u32),
    Offset(i32),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CellReference {
    pub sheet: Option<String>,
    pub row: RowRef,
    pub col: ColRef,
    pub spill: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RangeReference {
    pub sheet: Option<String>,
    pub start: CellReference,
    pub end: CellReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOperator {
    Plus,
    Minus,
    Percent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Concat,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaParseError {
    pub pos: usize,
    pub message: String,
}

impl fmt::Display for FormulaParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "formula parse error at {}: {}", self.pos, self.message)
    }
}

impl std::error::Error for FormulaParseError {}

pub fn parse_formula(formula: &str) -> Result<FormulaExpr, FormulaParseError> {
    let s = formula.trim();
    let s = s.strip_prefix('=').unwrap_or(s);
    let mut p = Parser::new(s);
    let expr = p.parse_expr(0)?;
    p.skip_ws();
    if !p.eof() {
        return Err(p.err("trailing characters"));
    }
    Ok(expr)
}

#[derive(Clone, Copy)]
enum ShiftMode {
    RelativeOnly,
    All,
}

impl FormulaExpr {
    pub fn canonicalize(&self) -> FormulaExpr {
        let mut e = self.clone();
        e.canonicalize_in_place();
        e
    }

    pub fn shifted(&self, row_shift: i32, col_shift: i32, mode: ShiftMode) -> FormulaExpr {
        shift_expr(self, row_shift, col_shift, mode)
    }

    fn canonicalize_in_place(&mut self) {
        match self {
            FormulaExpr::FunctionCall { name, args } => {
                *name = name.to_ascii_uppercase();
                for a in args.iter_mut() {
                    a.canonicalize_in_place();
                }
                if is_commutative_function(name) {
                    args.sort_by_key(|a| canonical_sort_key(a));
                }
            }
            FormulaExpr::NamedRef(name) => {
                *name = name.to_ascii_uppercase();
            }
            FormulaExpr::CellRef(r) => {
                if let Some(s) = &mut r.sheet {
                    *s = s.to_ascii_uppercase();
                }
            }
            FormulaExpr::RangeRef(r) => {
                if let Some(s) = &mut r.sheet {
                    *s = s.to_ascii_uppercase();
                }
                if let Some(s) = &mut r.start.sheet {
                    *s = s.to_ascii_uppercase();
                }
                if let Some(s) = &mut r.end.sheet {
                    *s = s.to_ascii_uppercase();
                }
                let a = ref_sort_key(&r.start);
                let b = ref_sort_key(&r.end);
                if b < a {
                    std::mem::swap(&mut r.start, &mut r.end);
                }
            }
            FormulaExpr::UnaryOp { operand, .. } => {
                operand.canonicalize_in_place();
            }
            FormulaExpr::BinaryOp { op, left, right } => {
                left.canonicalize_in_place();
                right.canonicalize_in_place();
                if is_commutative_binary(*op) {
                    let lk = canonical_sort_key(left);
                    let rk = canonical_sort_key(right);
                    if rk < lk {
                        std::mem::swap(left, right);
                    }
                }
            }
            FormulaExpr::Array(rows) => {
                for row in rows.iter_mut() {
                    for cell in row.iter_mut() {
                        cell.canonicalize_in_place();
                    }
                }
            }
            _ => {}
        }
    }
}

fn is_commutative_function(name: &str) -> bool {
    matches!(name, "SUM" | "PRODUCT" | "MIN" | "MAX" | "AND" | "OR")
}

fn is_commutative_binary(op: BinaryOperator) -> bool {
    matches!(op, BinaryOperator::Add | BinaryOperator::Mul | BinaryOperator::Eq | BinaryOperator::Ne)
}

fn canonical_sort_key(e: &FormulaExpr) -> String {
    format!("{:?}", e.canonicalize())
}

fn ref_sort_key(r: &CellReference) -> (i64, i64, u8, u8) {
    (row_key(r.row), col_key(r.col), abs_key_row(r.row), abs_key_col(r.col))
}

fn row_key(r: RowRef) -> i64 {
    match r {
        RowRef::Absolute(n) | RowRef::Relative(n) => n as i64,
        RowRef::Offset(n) => n as i64,
    }
}

fn col_key(c: ColRef) -> i64 {
    match c {
        ColRef::Absolute(n) | ColRef::Relative(n) => n as i64,
        ColRef::Offset(n) => n as i64,
    }
}

fn abs_key_row(r: RowRef) -> u8 {
    match r {
        RowRef::Absolute(_) => 0,
        RowRef::Relative(_) => 1,
        RowRef::Offset(_) => 2,
    }
}

fn abs_key_col(c: ColRef) -> u8 {
    match c {
        ColRef::Absolute(_) => 0,
        ColRef::Relative(_) => 1,
        ColRef::Offset(_) => 2,
    }
}

fn shift_expr(e: &FormulaExpr, row_shift: i32, col_shift: i32, mode: ShiftMode) -> FormulaExpr {
    match e {
        FormulaExpr::CellRef(r) => FormulaExpr::CellRef(shift_cell_ref(r, row_shift, col_shift, mode)),
        FormulaExpr::RangeRef(r) => {
            let mut rr = r.clone();
            rr.start = shift_cell_ref(&rr.start, row_shift, col_shift, mode);
            rr.end = shift_cell_ref(&rr.end, row_shift, col_shift, mode);
            FormulaExpr::RangeRef(rr)
        }
        FormulaExpr::FunctionCall { name, args } => FormulaExpr::FunctionCall {
            name: name.clone(),
            args: args.iter().map(|a| shift_expr(a, row_shift, col_shift, mode)).collect(),
        },
        FormulaExpr::UnaryOp { op, operand } => FormulaExpr::UnaryOp {
            op: *op,
            operand: Box::new(shift_expr(operand, row_shift, col_shift, mode)),
        },
        FormulaExpr::BinaryOp { op, left, right } => FormulaExpr::BinaryOp {
            op: *op,
            left: Box::new(shift_expr(left, row_shift, col_shift, mode)),
            right: Box::new(shift_expr(right, row_shift, col_shift, mode)),
        },
        FormulaExpr::Array(rows) => FormulaExpr::Array(
            rows.iter()
                .map(|row| row.iter().map(|x| shift_expr(x, row_shift, col_shift, mode)).collect())
                .collect(),
        ),
        _ => e.clone(),
    }
}

fn shift_cell_ref(r: &CellReference, row_shift: i32, col_shift: i32, mode: ShiftMode) -> CellReference {
    let mut out = r.clone();
    out.row = shift_row_ref(r.row, row_shift, mode);
    out.col = shift_col_ref(r.col, col_shift, mode);
    out
}

fn shift_row_ref(r: RowRef, delta: i32, mode: ShiftMode) -> RowRef {
    match r {
        RowRef::Relative(n) => RowRef::Relative(shift_u32(n, delta)),
        RowRef::Absolute(n) => match mode {
            ShiftMode::RelativeOnly => RowRef::Absolute(n),
            ShiftMode::All => RowRef::Absolute(shift_u32(n, delta)),
        },
        RowRef::Offset(n) => RowRef::Offset(n),
    }
}

fn shift_col_ref(c: ColRef, delta: i32, mode: ShiftMode) -> ColRef {
    match c {
        ColRef::Relative(n) => ColRef::Relative(shift_u32(n, delta)),
        ColRef::Absolute(n) => match mode {
            ShiftMode::RelativeOnly => ColRef::Absolute(n),
            ShiftMode::All => ColRef::Absolute(shift_u32(n, delta)),
        },
        ColRef::Offset(n) => ColRef::Offset(n),
    }
}

fn shift_u32(n: u32, delta: i32) -> u32 {
    let v = n as i64 + delta as i64;
    if v <= 0 {
        0
    } else if v >= u32::MAX as i64 {
        u32::MAX
    } else {
        v as u32
    }
}

pub fn formulas_equivalent_modulo_shift(
    a: &FormulaExpr,
    b: &FormulaExpr,
    row_shift: i32,
    col_shift: i32,
) -> bool {
    let a_shifted = a.shifted(row_shift, col_shift, ShiftMode::RelativeOnly).canonicalize();
    let b_canon = b.canonicalize();
    a_shifted == b_canon
}

struct Parser<'a> {
    s: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { s: input.as_bytes(), pos: 0 }
    }

    fn eof(&self) -> bool {
        self.pos >= self.s.len()
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.pos += 1;
        }
    }

    fn err(&self, msg: &str) -> FormulaParseError {
        FormulaParseError { pos: self.pos, message: msg.to_string() }
    }

    fn parse_expr(&mut self, min_bp: u8) -> Result<FormulaExpr, FormulaParseError> {
        self.skip_ws();

        let mut lhs = if matches!(self.peek(), Some(b'+' | b'-')) {
            let op = match self.bump().unwrap() {
                b'+' => UnaryOperator::Plus,
                b'-' => UnaryOperator::Minus,
                _ => return Err(self.err("invalid unary op")),
            };
            let rhs = self.parse_expr(90)?;
            FormulaExpr::UnaryOp { op, operand: Box::new(rhs) }
        } else {
            self.parse_primary()?
        };

        loop {
            self.skip_ws();

            while matches!(self.peek(), Some(b'%')) {
                self.bump();
                lhs = FormulaExpr::UnaryOp { op: UnaryOperator::Percent, operand: Box::new(lhs) };
                self.skip_ws();
            }

            let (op, l_bp, r_bp) = match self.peek_infix_op() {
                Some(x) => x,
                None => break,
            };

            if l_bp < min_bp {
                break;
            }

            self.consume_infix_op(op)?;
            let rhs = self.parse_expr(r_bp)?;
            lhs = FormulaExpr::BinaryOp { op, left: Box::new(lhs), right: Box::new(rhs) };
        }

        Ok(lhs)
    }

    fn peek_infix_op(&self) -> Option<(BinaryOperator, u8, u8)> {
        let b = self.peek()?;
        match b {
            b'+' => Some((BinaryOperator::Add, 50, 51)),
            b'-' => Some((BinaryOperator::Sub, 50, 51)),
            b'*' => Some((BinaryOperator::Mul, 60, 61)),
            b'/' => Some((BinaryOperator::Div, 60, 61)),
            b'^' => Some((BinaryOperator::Pow, 70, 70)),
            b'&' => Some((BinaryOperator::Concat, 40, 41)),
            b'=' => Some((BinaryOperator::Eq, 30, 31)),
            b'<' => {
                if self.s.get(self.pos + 1) == Some(&b'=') {
                    Some((BinaryOperator::Le, 30, 31))
                } else if self.s.get(self.pos + 1) == Some(&b'>') {
                    Some((BinaryOperator::Ne, 30, 31))
                } else {
                    Some((BinaryOperator::Lt, 30, 31))
                }
            }
            b'>' => {
                if self.s.get(self.pos + 1) == Some(&b'=') {
                    Some((BinaryOperator::Ge, 30, 31))
                } else {
                    Some((BinaryOperator::Gt, 30, 31))
                }
            }
            _ => None,
        }
    }

    fn consume_infix_op(&mut self, op: BinaryOperator) -> Result<(), FormulaParseError> {
        match op {
            BinaryOperator::Le | BinaryOperator::Ge => {
                self.bump();
                if self.bump() != Some(b'=') {
                    return Err(self.err("expected '='"));
                }
            }
            BinaryOperator::Ne => {
                self.bump();
                if self.bump() != Some(b'>') {
                    return Err(self.err("expected '>'"));
                }
            }
            BinaryOperator::Lt | BinaryOperator::Gt => {
                self.bump();
            }
            _ => {
                self.bump();
            }
        }
        Ok(())
    }

    fn parse_primary(&mut self) -> Result<FormulaExpr, FormulaParseError> {
        self.skip_ws();
        match self.peek() {
            Some(b'(') => {
                self.bump();
                let e = self.parse_expr(0)?;
                self.skip_ws();
                if self.bump() != Some(b')') {
                    return Err(self.err("expected ')'"));
                }
                Ok(e)
            }
            Some(b'{') => self.parse_array(),
            Some(b'"') => self.parse_string(),
            Some(b'#') => self.parse_error(),
            Some(b'0'..=b'9') => {
                if self.looks_like_row_range() {
                    return self.parse_row_range(None);
                }
                self.parse_number()
            }
            Some(b'\'' | b'[') => self.parse_ref_or_name_with_optional_sheet(),
            Some(b'$' | b'A'..=b'Z' | b'a'..=b'z' | b'_') => self.parse_ref_or_name_with_optional_sheet(),
            _ => Err(self.err("unexpected token")),
        }
    }

    fn parse_ref_or_name_with_optional_sheet(&mut self) -> Result<FormulaExpr, FormulaParseError> {
        let start = self.pos;
        if let Some(sheet) = self.try_parse_sheet_prefix()? {
            return self.parse_ref_or_name(Some(sheet));
        }
        self.pos = start;
        self.parse_ref_or_name(None)
    }

    fn try_parse_sheet_prefix(&mut self) -> Result<Option<String>, FormulaParseError> {
        self.skip_ws();
        match self.peek() {
            Some(b'\'') => {
                let sheet = self.parse_quoted_sheet_name()?;
                if self.peek() == Some(b'!') {
                    self.bump();
                    return Ok(Some(sheet));
                }
                Ok(None)
            }
            Some(b'[') => {
                let start = self.pos;
                while let Some(b) = self.peek() {
                    if b == b'!' {
                        let sheet = std::str::from_utf8(&self.s[start..self.pos]).unwrap().to_string();
                        self.bump();
                        return Ok(Some(sheet));
                    }
                    self.pos += 1;
                }
                self.pos = start;
                Ok(None)
            }
            _ => {
                let start = self.pos;
                let ident = self.parse_identifier()?;
                self.skip_ws();
                if self.peek() == Some(b'!') {
                    self.bump();
                    return Ok(Some(ident));
                }
                self.pos = start;
                Ok(None)
            }
        }
    }

    fn parse_ref_or_name(&mut self, sheet: Option<String>) -> Result<FormulaExpr, FormulaParseError> {
        self.skip_ws();

        if matches!(self.peek(), Some(b'0'..=b'9')) && self.looks_like_row_range() {
            return self.parse_row_range(sheet);
        }

        if matches!(self.peek(), Some(b'R' | b'r')) {
            let start = self.pos;
            if let Ok(r) = self.try_parse_r1c1(sheet.clone()) {
                return Ok(r);
            }
            self.pos = start;
        }

        if matches!(self.peek(), Some(b'$' | b'A'..=b'Z' | b'a'..=b'z')) {
            let start = self.pos;
            if let Some(r) = self.try_parse_a1_cell_ref(sheet.clone())? {
                let mut expr = FormulaExpr::CellRef(r);
                self.skip_ws();
                if self.peek() == Some(b':') {
                    self.bump();
                    let rhs = self.try_parse_a1_cell_ref(None)?
                        .ok_or_else(|| self.err("expected cell ref after ':'"))?;
                    expr = FormulaExpr::RangeRef(RangeReference {
                        sheet,
                        start: match expr {
                            FormulaExpr::CellRef(c) => c,
                            _ => unreachable!(),
                        },
                        end: rhs,
                    });
                }
                return Ok(expr);
            }
            self.pos = start;
        }

        let name = self.parse_identifier()?;
        self.skip_ws();

        if self.peek() == Some(b'[') {
            let structured = self.parse_bracket_blob()?;
            return Ok(FormulaExpr::NamedRef(format!("{}{}", name, structured)));
        }

        if self.peek() == Some(b'(') {
            self.bump();
            let mut args = Vec::new();
            self.skip_ws();
            if self.peek() != Some(b')') {
                loop {
                    let a = self.parse_expr(0)?;
                    args.push(a);
                    self.skip_ws();
                    match self.peek() {
                        Some(b',' | b';') => {
                            self.bump();
                            self.skip_ws();
                        }
                        _ => break,
                    }
                }
            }
            self.skip_ws();
            if self.bump() != Some(b')') {
                return Err(self.err("expected ')' after function args"));
            }
            return Ok(FormulaExpr::FunctionCall { name, args });
        }

        let upper = name.to_ascii_uppercase();
        if upper == "TRUE" {
            return Ok(FormulaExpr::Boolean(true));
        }
        if upper == "FALSE" {
            return Ok(FormulaExpr::Boolean(false));
        }

        Ok(FormulaExpr::NamedRef(name))
    }

    fn parse_number(&mut self) -> Result<FormulaExpr, FormulaParseError> {
        let start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')) {
            self.pos += 1;
        }
        let txt = std::str::from_utf8(&self.s[start..self.pos]).unwrap();
        let n = txt.parse::<f64>().map_err(|_| self.err("invalid number"))?;
        Ok(FormulaExpr::Number(n))
    }

    fn parse_string(&mut self) -> Result<FormulaExpr, FormulaParseError> {
        if self.bump() != Some(b'"') {
            return Err(self.err("expected '\"'"));
        }
        let mut out: Vec<u8> = Vec::new();
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

    fn parse_error(&mut self) -> Result<FormulaExpr, FormulaParseError> {
        let start = self.pos;
        self.bump();
        while let Some(b) = self.peek() {
            self.pos += 1;
            if b == b'!' {
                break;
            }
            if matches!(b, b',' | b';' | b')' | b'}') {
                break;
            }
        }
        let txt = std::str::from_utf8(&self.s[start..self.pos]).unwrap().to_string();
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

    fn parse_array(&mut self) -> Result<FormulaExpr, FormulaParseError> {
        if self.bump() != Some(b'{') {
            return Err(self.err("expected '{'"));
        }
        let mut rows: Vec<Vec<FormulaExpr>> = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.bump();
            return Ok(FormulaExpr::Array(rows));
        }
        loop {
            let mut row: Vec<FormulaExpr> = Vec::new();
            loop {
                let e = self.parse_expr(0)?;
                row.push(e);
                self.skip_ws();
                match self.peek() {
                    Some(b',') => {
                        self.bump();
                        self.skip_ws();
                    }
                    _ => break,
                }
            }
            rows.push(row);
            self.skip_ws();
            match self.peek() {
                Some(b';') => {
                    self.bump();
                    self.skip_ws();
                    continue;
                }
                Some(b'}') => {
                    self.bump();
                    break;
                }
                _ => return Err(self.err("expected ';' or '}' in array")),
            }
        }
        Ok(FormulaExpr::Array(rows))
    }

    fn parse_quoted_sheet_name(&mut self) -> Result<String, FormulaParseError> {
        if self.bump() != Some(b'\'') {
            return Err(self.err("expected quote"));
        }
        let mut out: Vec<u8> = Vec::new();
        loop {
            match self.bump() {
                Some(b'\'') => {
                    if self.peek() == Some(b'\'') {
                        self.bump();
                        out.push(b'\'');
                        continue;
                    }
                    break;
                }
                Some(b) => out.push(b),
                None => return Err(self.err("unterminated sheet name")),
            }
        }
        Ok(String::from_utf8(out).unwrap_or_default())
    }

    fn parse_identifier(&mut self) -> Result<String, FormulaParseError> {
        self.skip_ws();
        let start = self.pos;
        let Some(b0) = self.peek() else { return Err(self.err("expected identifier")); };
        if !is_ident_start(b0) {
            return Err(self.err("expected identifier"));
        }
        self.pos += 1;
        while let Some(b) = self.peek() {
            if !is_ident_continue(b) {
                break;
            }
            self.pos += 1;
        }
        Ok(std::str::from_utf8(&self.s[start..self.pos]).unwrap().to_string())
    }

    fn parse_bracket_blob(&mut self) -> Result<String, FormulaParseError> {
        self.skip_ws();
        if self.peek() != Some(b'[') {
            return Err(self.err("expected '['"));
        }
        let start = self.pos;
        let mut depth = 0i32;
        while let Some(b) = self.bump() {
            if b == b'[' {
                depth += 1;
            } else if b == b']' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
        }
        if depth != 0 {
            return Err(self.err("unterminated structured reference"));
        }
        Ok(std::str::from_utf8(&self.s[start..self.pos]).unwrap().to_string())
    }

    fn try_parse_r1c1(&mut self, sheet: Option<String>) -> Result<FormulaExpr, FormulaParseError> {
        let start = self.pos;
        let r = self.bump().unwrap();
        if r != b'R' && r != b'r' {
            self.pos = start;
            return Err(self.err("not R1C1"));
        }

        let row = self.parse_r1c1_part_row()?;
        let c = self.bump().ok_or_else(|| self.err("expected 'C'"))?;
        if c != b'C' && c != b'c' {
            self.pos = start;
            return Err(self.err("not R1C1"));
        }
        let col = self.parse_r1c1_part_col()?;

        Ok(FormulaExpr::CellRef(CellReference {
            sheet,
            row,
            col,
            spill: false,
        }))
    }

    fn parse_r1c1_part_row(&mut self) -> Result<RowRef, FormulaParseError> {
        match self.peek() {
            Some(b'[') => {
                self.bump();
                let n = self.parse_signed_int()?;
                if self.bump() != Some(b']') {
                    return Err(self.err("expected ']'"));
                }
                Ok(RowRef::Offset(n))
            }
            Some(b'0'..=b'9') => Ok(RowRef::Absolute(self.parse_u32()?)),
            _ => Ok(RowRef::Offset(0)),
        }
    }

    fn parse_r1c1_part_col(&mut self) -> Result<ColRef, FormulaParseError> {
        match self.peek() {
            Some(b'[') => {
                self.bump();
                let n = self.parse_signed_int()?;
                if self.bump() != Some(b']') {
                    return Err(self.err("expected ']'"));
                }
                Ok(ColRef::Offset(n))
            }
            Some(b'0'..=b'9') => Ok(ColRef::Absolute(self.parse_u32()?)),
            _ => Ok(ColRef::Offset(0)),
        }
    }

    fn parse_signed_int(&mut self) -> Result<i32, FormulaParseError> {
        self.skip_ws();
        let start = self.pos;
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.pos += 1;
        }
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        let txt = std::str::from_utf8(&self.s[start..self.pos]).unwrap();
        txt.parse::<i32>().map_err(|_| self.err("invalid signed int"))
    }

    fn parse_u32(&mut self) -> Result<u32, FormulaParseError> {
        self.skip_ws();
        let start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        let txt = std::str::from_utf8(&self.s[start..self.pos]).unwrap();
        txt.parse::<u32>().map_err(|_| self.err("invalid number"))
    }

    fn try_parse_a1_cell_ref(&mut self, sheet: Option<String>) -> Result<Option<CellReference>, FormulaParseError> {
        self.skip_ws();
        let start = self.pos;

        let col_abs = self.consume_if(b'$');
        if matches!(self.peek(), Some(b'R' | b'r')) {
            if self.looks_like_r1c1() {
                self.pos = start;
                return Ok(None);
            }
        }

        let col_start = self.pos;
        while matches!(self.peek(), Some(b'A'..=b'Z' | b'a'..=b'z')) {
            self.pos += 1;
        }
        if self.pos == col_start {
            self.pos = start;
            return Ok(None);
        }

        let col_txt = std::str::from_utf8(&self.s[col_start..self.pos]).unwrap();
        if col_txt.len() > 3 {
            self.pos = start;
            return Ok(None);
        }

        let col_num = col_letters_to_u32(col_txt).ok_or_else(|| self.err("invalid column"))?;
        let row_abs = self.consume_if(b'$');

        let row_start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        if self.pos == row_start {
            self.pos = start;
            return Ok(None);
        }

        let row_txt = std::str::from_utf8(&self.s[row_start..self.pos]).unwrap();
        let row_num = row_txt.parse::<u32>().map_err(|_| self.err("invalid row"))?;

        let mut spill = false;
        if self.peek() == Some(b'#') {
            self.bump();
            spill = true;
        }

        Ok(Some(CellReference {
            sheet,
            row: if row_abs { RowRef::Absolute(row_num) } else { RowRef::Relative(row_num) },
            col: if col_abs { ColRef::Absolute(col_num) } else { ColRef::Relative(col_num) },
            spill,
        }))
    }

    fn consume_if(&mut self, b: u8) -> bool {
        if self.peek() == Some(b) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn looks_like_r1c1(&self) -> bool {
        let mut i = self.pos;
        if i >= self.s.len() { return false; }
        let b = self.s[i];
        if b != b'R' && b != b'r' { return false; }
        i += 1;
        while i < self.s.len() && matches!(self.s[i], b'0'..=b'9') {
            i += 1;
        }
        if i < self.s.len() && self.s[i] == b'[' {
            return true;
        }
        i < self.s.len() && (self.s[i] == b'C' || self.s[i] == b'c')
    }

    fn looks_like_row_range(&self) -> bool {
        let mut i = self.pos;
        while i < self.s.len() && matches!(self.s[i], b'0'..=b'9') {
            i += 1;
        }
        if i == self.pos { return false; }
        while i < self.s.len() && matches!(self.s[i], b' ' | b'\t') { i += 1; }
        if i >= self.s.len() || self.s[i] != b':' { return false; }
        i += 1;
        while i < self.s.len() && matches!(self.s[i], b' ' | b'\t') { i += 1; }
        let j = i;
        while i < self.s.len() && matches!(self.s[i], b'0'..=b'9') {
            i += 1;
        }
        i > j
    }

    fn parse_row_range(&mut self, sheet: Option<String>) -> Result<FormulaExpr, FormulaParseError> {
        let start_row = self.parse_u32()?;
        self.skip_ws();
        if self.bump() != Some(b':') {
            return Err(self.err("expected ':' in row range"));
        }
        self.skip_ws();
        let end_row = self.parse_u32()?;
        Ok(FormulaExpr::NamedRef(format!(
            "{}{}:{}",
            match sheet {
                Some(s) => format!("{}!", s),
                None => "".to_string(),
            },
            start_row,
            end_row
        )))
    }
}

fn is_ident_start(b: u8) -> bool {
    matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'_' | b'\\')
}

fn is_ident_continue(b: u8) -> bool {
    is_ident_start(b) || matches!(b, b'0'..=b'9' | b'.')
}

fn col_letters_to_u32(s: &str) -> Option<u32> {
    let mut col: u32 = 0;
    for b in s.bytes() {
        let c = b.to_ascii_uppercase();
        if !(b'A'..=b'Z').contains(&c) {
            return None;
        }
        col = col * 26 + (c - b'A' + 1) as u32;
    }
    Some(col)
}
```

This gives you:

* AST + parser
* Canonicalization that normalizes function/name case and sorts commutative args/ops
* Shifted equivalence helper `formulas_equivalent_modulo_shift` 

It is intentionally conservative: if it can’t parse a given construct perfectly, it tends to represent it as `NamedRef(...)` rather than producing incorrect structure. That keeps semantic classification stable.

---

### 1.3 Parser tests

Branch 7.1 explicitly asks for comprehensive parser tests . Create `core/tests/f7_formula_parser_tests.rs` with table-driven cases:

* Literals: `1`, `"x"`, `TRUE`, `#DIV/0!`
* A1 refs: `A1`, `$B$2`, `Sheet1!A1`, `'My Sheet'!A1`
* R1C1 refs: `R1C1`, `R[1]C[-1]`
* Functions: `SUM(A1,B1)`, nested calls
* Arrays: `{1,2;3,4}`

You only need to assert “parses successfully” for many, and precise AST equality for a smaller representative set.

---

## Step 2 — Branch 7.2: Canonicalization + hashing hooks

Branch 7.2 expects:

* Normalize case/whitespace
* Canonicalize commutative functions (`SUM`, `MIN`, `MAX`, `AND`, `OR`) by sorting args
* Support stable comparison/hashing 

The `canonicalize()` method in the file above already covers the “canonicalization” part. Two additional upgrades are worth adding before integration:

1. **Avoid `format!("{:?}", ...)` as sort key**
   It’s correct but can be slow. Instead, add a lightweight canonical hash (like M does via xxhash) and sort by that. This is exactly the approach you already use for M AST semantic hashing .

2. **Use canonical hash in commutative ordering**
   So “semantic order” is deterministic without expensive stringification.

If you want to keep Branch 7 scoped, you can do this as a “phase 2 within 7.2” after tests pass.

---

## Step 3 — Branch 7.3: Shift equivalence (“filled down/across”)

Branch 7.3 requires:

* `formulas_equivalent_modulo_shift(formula1, formula2, row_shift, col_shift) -> bool`
* Tests like `A1+B1` in `C1` vs `A2+B2` in `C2` = equivalent for row shift 1 

You already have that function in the new module above, implemented as:

* shift (relative refs only)
* canonicalize
* compare

Add tests in `core/tests/f7_formula_shift_tests.rs`:

* Equivalent: old `A1+B1`, new `A2+B2`, row_shift=1 col_shift=0
* Not equivalent: old `A1+B1`, new `A1+B2`, shift 0

---

## Step 4 — Branch 7.4: Integrate into `CellEdited` with feature flag

Branch 7.4 expects:

* `diff_cell_formulas(old, new, config) -> FormulaDiffResult` 
* `DiffOp::CellEdited` updated to include classification 
* Feature flag `enable_formula_semantic_diff` (you already have it) 

### 4.1 Add `FormulaDiffResult` and add it to `DiffOp::CellEdited`

#### `core/src/diff.rs`

**Code to replace (current `CellEdited` variant):** 

```rust
CellEdited {
    sheet: SheetId,
    addr: CellAddress,
    from: CellSnapshot,
    to: CellSnapshot,
},
```

**New code to replace it with:**

```rust
CellEdited {
    sheet: SheetId,
    addr: CellAddress,
    from: CellSnapshot,
    to: CellSnapshot,
    #[serde(default)]
    formula_diff: FormulaDiffResult,
},
```

Now add the enum definition near other diff-classification types (similar to `QueryChangeKind`):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormulaDiffResult {
    Unknown,
    Unchanged,
    Added,
    Removed,
    FormattingOnly,
    Filled,
    SemanticChange,
    TextChange,
}

impl Default for FormulaDiffResult {
    fn default() -> Self {
        FormulaDiffResult::Unknown
    }
}
```

Now update the constructor helper.

**Code to replace (current helper):** 

```rust
pub fn cell_edited(sheet: SheetId, addr: CellAddress, from: CellSnapshot, to: CellSnapshot) -> DiffOp {
    debug_assert_eq!(from.addr, addr);
    debug_assert_eq!(to.addr, addr);
    DiffOp::CellEdited { sheet, addr, from, to }
}
```

**New code to replace it with:**

```rust
pub fn cell_edited(
    sheet: SheetId,
    addr: CellAddress,
    from: CellSnapshot,
    to: CellSnapshot,
    formula_diff: FormulaDiffResult,
) -> DiffOp {
    debug_assert_eq!(from.addr, addr);
    debug_assert_eq!(to.addr, addr);
    DiffOp::CellEdited {
        sheet,
        addr,
        from,
        to,
        formula_diff,
    }
}
```

This will ripple through call sites (engine + tests), but the signature change is useful: it forces every emission site to be deliberate about classification.

---

### 4.2 Implement `diff_cell_formulas` with StringId inputs

Branch 7.4’s signature uses `Option<String>` , but your cells carry `Option<StringId>` . The clean adapter is:

* `diff_cell_formulas_ids(pool, old_id, new_id, row_shift, col_shift, config, cache)`

Create `core/src/formula_diff.rs`:

```rust
use rustc_hash::FxHashMap;

use crate::config::DiffConfig;
use crate::diff::FormulaDiffResult;
use crate::formula::{parse_formula, formulas_equivalent_modulo_shift, FormulaExpr};
use crate::string_pool::{StringId, StringPool};

#[derive(Default)]
pub(crate) struct FormulaParseCache {
    parsed: FxHashMap<StringId, Option<FormulaExpr>>,
    canonical: FxHashMap<StringId, Option<FormulaExpr>>,
}

impl FormulaParseCache {
    fn parsed(&mut self, pool: &StringPool, id: StringId) -> Option<&FormulaExpr> {
        if !self.parsed.contains_key(&id) {
            let s = pool.resolve(id);
            self.parsed.insert(id, parse_formula(s).ok());
        }
        self.parsed.get(&id).and_then(|x| x.as_ref())
    }

    fn canonical(&mut self, pool: &StringPool, id: StringId) -> Option<FormulaExpr> {
        if !self.canonical.contains_key(&id) {
            let canon = self.parsed(pool, id).map(|e| e.canonicalize());
            self.canonical.insert(id, canon);
        }
        self.canonical.get(&id).and_then(|x| x.clone())
    }
}

pub(crate) fn diff_cell_formulas_ids(
    pool: &StringPool,
    cache: &mut FormulaParseCache,
    old: Option<StringId>,
    new: Option<StringId>,
    row_shift: i32,
    col_shift: i32,
    config: &DiffConfig,
) -> FormulaDiffResult {
    if old == new {
        return FormulaDiffResult::Unchanged;
    }

    match (old, new) {
        (None, Some(_)) => return FormulaDiffResult::Added,
        (Some(_), None) => return FormulaDiffResult::Removed,
        (None, None) => return FormulaDiffResult::Unchanged,
        _ => {}
    }

    if !config.enable_formula_semantic_diff {
        return FormulaDiffResult::TextChange;
    }

    let (Some(old_id), Some(new_id)) = (old, new) else {
        return FormulaDiffResult::TextChange;
    };

    let old_ast = match cache.parsed(pool, old_id) {
        Some(a) => a.clone(),
        None => return FormulaDiffResult::TextChange,
    };
    let new_ast = match cache.parsed(pool, new_id) {
        Some(a) => a.clone(),
        None => return FormulaDiffResult::TextChange,
    };

    let old_c = old_ast.canonicalize();
    let new_c = match cache.canonical(pool, new_id) {
        Some(c) => c,
        None => new_ast.canonicalize(),
    };

    if old_c == new_c {
        return FormulaDiffResult::FormattingOnly;
    }

    if row_shift != 0 || col_shift != 0 {
        if formulas_equivalent_modulo_shift(&old_ast, &new_ast, row_shift, col_shift) {
            return FormulaDiffResult::Filled;
        }
    }

    FormulaDiffResult::SemanticChange
}
```

This matches Branch 7.4 behavior: if parse fails, fall back to “text change” , and it is gated by the feature flag .

---

### 4.3 Wire formula diff into all `CellEdited` emissions

This is the most mechanical part. Everywhere the engine currently does:

```rust
DiffOp::cell_edited(sheet_id.clone(), addr, from, to)
```

…you’ll:

1. Compute `(row_shift, col_shift)` from the alignment mapping (old indices vs new indices)
2. Compute `formula_diff = diff_cell_formulas_ids(...)`
3. Pass it into `DiffOp::cell_edited(..., formula_diff)`

#### Update example: `diff_row_pair_sparse`

Right now it only receives `row_b` and thus cannot compute `row_shift`. That’s a direct blocker for Branch 7.3 integration, so change it to accept both `row_a` and `row_b`.

**Code to replace (existing signature and emission, excerpt):** 

```rust
fn diff_row_pair_sparse(
    sheet_id: &SheetId,
    row_b: u32,
    overlap_cols: u32,
    old_cells: &[(u32, &Cell)],
    new_cells: &[(u32, &Cell)],
    sink: &mut impl DiffSink,
    op_count: &mut usize,
    config: &DiffConfig,
) -> Result<u64, DiffError> {
    ...
            emit_op(
                sink,
                op_count,
                DiffOp::cell_edited(sheet_id.clone(), addr, from, to),
            )?;
```

**New code to replace it with:**

```rust
fn diff_row_pair_sparse(
    sheet_id: &SheetId,
    pool: &StringPool,
    formula_cache: &mut crate::formula_diff::FormulaParseCache,
    row_a: u32,
    row_b: u32,
    overlap_cols: u32,
    old_cells: &[(u32, &Cell)],
    new_cells: &[(u32, &Cell)],
    sink: &mut impl DiffSink,
    op_count: &mut usize,
    config: &DiffConfig,
) -> Result<u64, DiffError> {
    let mut i = 0usize;
    let mut j = 0usize;
    let mut compared = 0u64;

    let row_shift = row_b as i32 - row_a as i32;

    while i < old_cells.len() || j < new_cells.len() {
        let col_a = old_cells.get(i).map(|(c, _)| *c).unwrap_or(u32::MAX);
        let col_b = new_cells.get(j).map(|(c, _)| *c).unwrap_or(u32::MAX);
        let col = col_a.min(col_b);

        if col >= overlap_cols {
            break;
        }

        compared = compared.saturating_add(1);

        let old_cell = if col_a == col {
            let (_, cell) = old_cells[i];
            i += 1;
            Some(cell)
        } else {
            None
        };

        let new_cell = if col_b == col {
            let (_, cell) = new_cells[j];
            j += 1;
            Some(cell)
        } else {
            None
        };

        let changed = !cells_content_equal(old_cell, new_cell);

        if changed || config.include_unchanged_cells {
            let addr = CellAddress::from_indices(row_b, col);
            let from = snapshot_with_addr(old_cell, addr);
            let to = snapshot_with_addr(new_cell, addr);

            let old_f = old_cell.and_then(|c| c.formula);
            let new_f = new_cell.and_then(|c| c.formula);
            let formula_diff = crate::formula_diff::diff_cell_formulas_ids(
                pool,
                formula_cache,
                old_f,
                new_f,
                row_shift,
                0,
                config,
            );

            emit_op(
                sink,
                op_count,
                DiffOp::cell_edited(sheet_id.clone(), addr, from, to, formula_diff),
            )?;
        }
    }

    Ok(compared)
}
```

Now you must update all call sites of `diff_row_pair_sparse` to pass:

* `pool`
* `formula_cache`
* `row_a` and `row_b`

For example:

* `emit_moved_row_block_edits` currently calls it with only `mv.dst_start_row + offset` ; it also has `old_idx` (source row) so you can pass both.
* `emit_aligned_diffs` loops `for (row_a, row_b)` ; pass both.
* Any AMR-alignment path with `(row_a, row_b)` pairs must pass both.

#### Apply the same pattern to other emission sites

From your context, you’ll need to adjust these hotspots:

* `diff_row_pair` (already has `row_a, row_b`) 
* `emit_column_aligned_diffs` (has `(col_a, col_b)` pairs; row_shift=0, col_shift=col_b-col_a) 
* `diff_aligned_with_masks` (has `(row_a,row_b)` and `(col_a,col_b)`; compute both shifts) 
* `positional_diff_with_masks` / `positional_diff_masked_equal_size` (shifts = 0) 

If you want to reduce duplication, introduce a tiny helper in `engine.rs`:

* Input: `old_cell`, `new_cell`, `(row_shift, col_shift)`, plus `pool`, `formula_cache`, `config`
* Output: `FormulaDiffResult`

This keeps every emission consistent.

---

### 4.4 Update tests that assert `CellEdited` shapes

A non-trivial number of tests pattern-match `DiffOp::CellEdited { sheet, addr, from, to }` without `..` (example shown in `g2_core_diff_tests` excerpt) . Those will need to accept `formula_diff` or use `..`.

Also, `pg4_diffop_tests` currently validates JSON keysets for `CellEdited` and will need to add `"formula_diff"` to the expected set .

And any tests constructing `DiffOp::cell_edited(...)` must pass the new argument (output JSON tests do this) .

---

## Step 5 — Add the Branch 7 integration tests

These are the tests that prove Branch 7 is “done” beyond unit tests.

### 5.1 Formatting-only diff is detected when enabled

Create a grid-only test (no xlsx fixtures needed) that changes formula text only by whitespace/case:

* old formula: `sum(a1,b1)`
* new formula: `SUM(A1,B1)`

Expect:

* a `CellEdited` occurs (text changed)
* `formula_diff == FormattingOnly` when flag enabled
* `formula_diff == TextChange` when flag disabled

### 5.2 Filled-down diff is detected on row shift

Create:

* Old sheet has `C1 = "A1+B1"`
* New sheet inserts a blank row above and has `C2 = "A2+B2"`.

Then row alignment should map old row 0 -> new row 1, emitting `RowAdded` and a `CellEdited` at `C2` with `formula_diff == Filled`. This matches Branch 7.3’s “fill detection” expectation .

---

## Step 6 — Performance and safety checks

Branch 7 requires the feature flag can disable semantic diff for performance . You already have the config field, so ensure these properties:

* When `enable_formula_semantic_diff == false`, `diff_cell_formulas_ids(...)` must do **no parsing** (fast path).
* Parsing should be cached by `StringId` so large sheets with repeated formulas do not explode CPU time.
* Keep schema backward-compatible by using `#[serde(default)]` on the new `formula_diff` field (old JSON reports still deserialize).

---

## Final “Definition of done” checklist for Branch 7

1. `core/src/formula.rs` exists with AST + parser and tests pass 
2. Canonicalization covers commutative functions/operators and is tested 
3. `formulas_equivalent_modulo_shift(...)` exists and tests match Branch 7.3 
4. `FormulaDiffResult` exists and `DiffOp::CellEdited` includes `formula_diff` 
5. Engine computes `(row_shift, col_shift)` at emission sites and populates `formula_diff`
6. Feature flag controls whether parsing/semantic diff runs 
7. JSON shape tests updated (`pg4_diffop_tests`) 
8. Integration tests: formatting-only + filled-down are green

---

If you want, I can also provide a “call-site punch list” (every function that emits `CellEdited` and the exact `row_shift/col_shift` you should use in each), but the snippets above already show the critical mechanical change: **carry alignment mapping indices down to where the cell edit is emitted**, so the formula diff logic has the shift context it needs.
