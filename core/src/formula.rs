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
pub(crate) enum ShiftMode {
    RelativeOnly,
    #[cfg(any(test, feature = "dev-apis"))]
    #[allow(dead_code)]
    All,
}

impl FormulaExpr {
    pub fn canonicalize(&self) -> FormulaExpr {
        let mut e = self.clone();
        e.canonicalize_in_place();
        e
    }

    pub(crate) fn shifted(&self, row_shift: i32, col_shift: i32, mode: ShiftMode) -> FormulaExpr {
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
    matches!(
        op,
        BinaryOperator::Add | BinaryOperator::Mul | BinaryOperator::Eq | BinaryOperator::Ne
    )
}

fn canonical_sort_key(e: &FormulaExpr) -> String {
    format!("{:?}", e.canonicalize())
}

fn ref_sort_key(r: &CellReference) -> (i64, i64, u8, u8) {
    (
        row_key(r.row),
        col_key(r.col),
        abs_key_row(r.row),
        abs_key_col(r.col),
    )
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
        FormulaExpr::CellRef(r) => {
            FormulaExpr::CellRef(shift_cell_ref(r, row_shift, col_shift, mode))
        }
        FormulaExpr::RangeRef(r) => {
            let mut rr = r.clone();
            rr.start = shift_cell_ref(&rr.start, row_shift, col_shift, mode);
            rr.end = shift_cell_ref(&rr.end, row_shift, col_shift, mode);
            FormulaExpr::RangeRef(rr)
        }
        FormulaExpr::FunctionCall { name, args } => FormulaExpr::FunctionCall {
            name: name.clone(),
            args: args
                .iter()
                .map(|a| shift_expr(a, row_shift, col_shift, mode))
                .collect(),
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
                .map(|row| {
                    row.iter()
                        .map(|x| shift_expr(x, row_shift, col_shift, mode))
                        .collect()
                })
                .collect(),
        ),
        _ => e.clone(),
    }
}

fn shift_cell_ref(
    r: &CellReference,
    row_shift: i32,
    col_shift: i32,
    mode: ShiftMode,
) -> CellReference {
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
            #[cfg(any(test, feature = "dev-apis"))]
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
            #[cfg(any(test, feature = "dev-apis"))]
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
    let a_shifted = a
        .shifted(row_shift, col_shift, ShiftMode::RelativeOnly)
        .canonicalize();
    let b_canon = b.canonicalize();
    a_shifted == b_canon
}

struct Parser<'a> {
    s: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            s: input.as_bytes(),
            pos: 0,
        }
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
        FormulaParseError {
            pos: self.pos,
            message: msg.to_string(),
        }
    }

    fn parse_expr(&mut self, min_bp: u8) -> Result<FormulaExpr, FormulaParseError> {
        self.skip_ws();

        let mut lhs = if matches!(self.peek(), Some(b'+' | b'-')) {
            let op_byte = self
                .bump()
                .ok_or_else(|| self.err("unexpected EOF after unary op"))?;
            let op = match op_byte {
                b'+' => UnaryOperator::Plus,
                b'-' => UnaryOperator::Minus,
                _ => return Err(self.err("invalid unary op")),
            };
            let rhs = self.parse_expr(90)?;
            FormulaExpr::UnaryOp {
                op,
                operand: Box::new(rhs),
            }
        } else {
            self.parse_primary()?
        };

        loop {
            self.skip_ws();

            while matches!(self.peek(), Some(b'%')) {
                self.bump();
                lhs = FormulaExpr::UnaryOp {
                    op: UnaryOperator::Percent,
                    operand: Box::new(lhs),
                };
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
            lhs = FormulaExpr::BinaryOp {
                op,
                left: Box::new(lhs),
                right: Box::new(rhs),
            };
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
            Some(b'$' | b'A'..=b'Z' | b'a'..=b'z' | b'_') => {
                self.parse_ref_or_name_with_optional_sheet()
            }
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
                        let sheet = std::str::from_utf8(&self.s[start..self.pos])
                            .map_err(|_| self.err("invalid utf-8 in sheet name"))?
                            .to_string();
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
                let Some(b) = self.peek() else {
                    return Ok(None);
                };
                if !is_ident_start(b) {
                    return Ok(None);
                }
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

    fn parse_ref_or_name(
        &mut self,
        sheet: Option<String>,
    ) -> Result<FormulaExpr, FormulaParseError> {
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
                    let start_ref = r.clone();
                    let mut expr = FormulaExpr::CellRef(r);
                    self.skip_ws();
                    if self.peek() == Some(b':') {
                        self.bump();
                        let rhs = self.try_parse_a1_cell_ref(None)?;
                        if let Some(end) = rhs {
                            expr = FormulaExpr::RangeRef(RangeReference {
                                sheet,
                                start: start_ref,
                                end,
                            });
                        }
                    }
                    return Ok(expr);
            }
            self.pos = start;
        }

        let ident = self.parse_identifier()?;
        self.skip_ws();

        if sheet.is_none()
            && ident.eq_ignore_ascii_case("TRUE")
            && !matches!(self.peek(), Some(b'(' | b'['))
        {
            return Ok(FormulaExpr::Boolean(true));
        }
        if sheet.is_none()
            && ident.eq_ignore_ascii_case("FALSE")
            && !matches!(self.peek(), Some(b'(' | b'['))
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
    }

    fn parse_array(&mut self) -> Result<FormulaExpr, FormulaParseError> {
        self.bump();
        let mut rows = Vec::new();
        let mut current_row = Vec::new();
        loop {
            self.skip_ws();
            if self.peek() == Some(b'}') {
                self.bump();
                if !current_row.is_empty() {
                    rows.push(current_row);
                }
                break;
            }
            let elem = self.parse_expr(0)?;
            current_row.push(elem);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.bump();
                }
                Some(b';') => {
                    self.bump();
                    rows.push(current_row);
                    current_row = Vec::new();
                }
                Some(b'}') => {
                    self.bump();
                    rows.push(current_row);
                    break;
                }
                _ => return Err(self.err("expected ',', ';', or '}'")),
            }
        }
        Ok(FormulaExpr::Array(rows))
    }

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

        let s = String::from_utf8(out).map_err(|_| self.err("invalid utf-8 in string"))?;
        Ok(FormulaExpr::Text(s))
    }

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

    fn parse_number(&mut self) -> Result<FormulaExpr, FormulaParseError> {
        let start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        let txt = std::str::from_utf8(&self.s[start..self.pos])
            .map_err(|_| self.err("invalid utf-8 in number"))?;
        let n: f64 = txt.parse().map_err(|_| self.err("invalid number"))?;
        Ok(FormulaExpr::Number(n))
    }

    fn parse_identifier(&mut self) -> Result<String, FormulaParseError> {
        self.skip_ws();
        let start = self.pos;
        if let Some(b) = self.peek() {
            if !is_ident_start(b) {
                return Err(self.err("expected identifier"));
            }
        } else {
            return Err(self.err("expected identifier"));
        }
        self.pos += 1;
        while let Some(b) = self.peek() {
            if is_ident_continue(b) {
                self.pos += 1;
            } else {
                break;
            }
        }
        let ident = std::str::from_utf8(&self.s[start..self.pos])
            .map_err(|_| self.err("invalid utf-8 in identifier"))?
            .to_string();
        Ok(ident)
    }

    fn parse_quoted_sheet_name(&mut self) -> Result<String, FormulaParseError> {
        debug_assert_eq!(self.peek(), Some(b'\''));
        self.bump();
        let start = self.pos;
        while let Some(b) = self.peek() {
            self.pos += 1;
            if b == b'\'' {
                if self.peek() == Some(b'\'') {
                    self.pos += 1;
                    continue;
                }
                let name = std::str::from_utf8(&self.s[start..self.pos - 1])
                    .map_err(|_| self.err("invalid utf-8 in sheet name"))?
                    .replace("''", "'");
                return Ok(name);
            }
        }
        Err(self.err("unterminated sheet name"))
    }

    fn try_parse_r1c1(&mut self, sheet: Option<String>) -> Result<FormulaExpr, FormulaParseError> {
        let start = self.pos;
        self.skip_ws();
        if !matches!(self.peek(), Some(b'R' | b'r')) {
            self.pos = start;
            return Err(self.err("expected R1C1 ref"));
        }
        self.bump();
        let row = if self.peek() == Some(b'[') {
            self.bump();
            let offset = self.parse_i32()?;
            if self.bump() != Some(b']') {
                return Err(self.err("expected ']'"));
            }
            RowRef::Offset(offset)
        } else if matches!(self.peek(), Some(b'0'..=b'9')) {
            RowRef::Absolute(self.parse_u32()?)
        } else {
            RowRef::Relative(0)
        };

        if !matches!(self.peek(), Some(b'C' | b'c')) {
            self.pos = start;
            return Err(self.err("expected 'C'"));
        }
        self.bump();

        let col = if self.peek() == Some(b'[') {
            self.bump();
            let offset = self.parse_i32()?;
            if self.bump() != Some(b']') {
                return Err(self.err("expected ']'"));
            }
            ColRef::Offset(offset)
        } else if matches!(self.peek(), Some(b'0'..=b'9')) {
            ColRef::Absolute(self.parse_u32()?)
        } else {
            ColRef::Relative(0)
        };

        Ok(FormulaExpr::CellRef(CellReference {
            sheet,
            row,
            col,
            spill: false,
        }))
    }

    fn parse_i32(&mut self) -> Result<i32, FormulaParseError> {
        self.skip_ws();
        let start = self.pos;
        if self.peek() == Some(b'-') || self.peek() == Some(b'+') {
            self.pos += 1;
        }
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        let txt = std::str::from_utf8(&self.s[start..self.pos])
            .map_err(|_| self.err("invalid utf-8 in signed int"))?;
        txt.parse::<i32>()
            .map_err(|_| self.err("invalid signed int"))
    }

    fn parse_u32(&mut self) -> Result<u32, FormulaParseError> {
        self.skip_ws();
        let start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        let txt = std::str::from_utf8(&self.s[start..self.pos])
            .map_err(|_| self.err("invalid utf-8 in number"))?;
        txt.parse::<u32>().map_err(|_| self.err("invalid number"))
    }

    fn try_parse_a1_cell_ref(
        &mut self,
        sheet: Option<String>,
    ) -> Result<Option<CellReference>, FormulaParseError> {
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

        let col_txt = std::str::from_utf8(&self.s[col_start..self.pos])
            .map_err(|_| self.err("invalid utf-8 in column"))?;
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

        let row_txt = std::str::from_utf8(&self.s[row_start..self.pos])
            .map_err(|_| self.err("invalid utf-8 in row"))?;
        let row_num = row_txt
            .parse::<u32>()
            .map_err(|_| self.err("invalid row"))?;

        let mut spill = false;
        if self.peek() == Some(b'#') {
            self.bump();
            spill = true;
        }

        Ok(Some(CellReference {
            sheet,
            row: if row_abs {
                RowRef::Absolute(row_num)
            } else {
                RowRef::Relative(row_num)
            },
            col: if col_abs {
                ColRef::Absolute(col_num)
            } else {
                ColRef::Relative(col_num)
            },
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
        if i >= self.s.len() {
            return false;
        }
        let b = self.s[i];
        if b != b'R' && b != b'r' {
            return false;
        }
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
        if i == self.pos {
            return false;
        }
        while i < self.s.len() && matches!(self.s[i], b' ' | b'\t') {
            i += 1;
        }
        if i >= self.s.len() || self.s[i] != b':' {
            return false;
        }
        i += 1;
        while i < self.s.len() && matches!(self.s[i], b' ' | b'\t') {
            i += 1;
        }
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
