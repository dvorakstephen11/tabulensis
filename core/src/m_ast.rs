use std::hash::{Hash, Hasher};
use std::iter::Peekable;
use std::str::Chars;

use thiserror::Error;

mod step_model;
#[allow(unused_imports)]
pub(crate) use step_model::{
    extract_steps, ColumnTypeChange as StepColumnTypeChange, Extracted as StepExtracted, MStep,
    RenamePair as StepRenamePair, StepKind, StepPipeline,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MModuleAst {
    root: MExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MAstKind {
    Let {
        binding_count: usize,
    },
    Record {
        field_count: usize,
    },
    List {
        item_count: usize,
    },
    FunctionCall {
        name: String,
        arg_count: usize,
    },
    FunctionLiteral {
        param_count: usize,
    },
    UnaryOp,
    BinaryOp,
    TypeAscription,
    TryOtherwise,
    Primitive,
    Ident {
        name: String,
    },
    If,
    Each,
    Access {
        kind: MAstAccessKind,
        chain_len: usize,
    },
    Opaque {
        token_count: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MAstAccessKind {
    Field,
    Item,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum AccessKind {
    Field,
    Item,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum MUnaryOp {
    Not,
    Plus,
    Minus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum MBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Concat,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct MTypeRef {
    name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct MParam {
    name: String,
    ty: Option<MTypeRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum MExpr {
    Let {
        bindings: Vec<LetBinding>,
        body: Box<MExpr>,
    },
    Record {
        fields: Vec<RecordField>,
    },
    List {
        items: Vec<MExpr>,
    },
    FunctionCall {
        name: String,
        args: Vec<MExpr>,
    },
    FunctionLiteral {
        params: Vec<MParam>,
        return_type: Option<MTypeRef>,
        body: Box<MExpr>,
    },
    UnaryOp {
        op: MUnaryOp,
        expr: Box<MExpr>,
    },
    BinaryOp {
        op: MBinaryOp,
        left: Box<MExpr>,
        right: Box<MExpr>,
    },
    TypeAscription {
        expr: Box<MExpr>,
        ty: MTypeRef,
    },
    TryOtherwise {
        expr: Box<MExpr>,
        otherwise: Box<MExpr>,
    },
    Ident {
        name: String,
    },
    If {
        cond: Box<MExpr>,
        then_branch: Box<MExpr>,
        else_branch: Box<MExpr>,
    },
    Each {
        body: Box<MExpr>,
    },
    Access {
        base: Box<MExpr>,
        kind: AccessKind,
        key: Box<MExpr>,
    },
    Primitive(MPrimitive),
    Opaque(Vec<MToken>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct LetBinding {
    name: String,
    value: Box<MExpr>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct RecordField {
    name: String,
    value: Box<MExpr>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum MPrimitive {
    String(String),
    Number(String),
    Boolean(bool),
    Null,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum MToken {
    KeywordLet,
    KeywordIn,
    KeywordIf,
    KeywordThen,
    KeywordElse,
    KeywordEach,
    Identifier(String),
    StringLiteral(String),
    Number(String),
    Symbol(char),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MTokenDebug {
    KeywordLet,
    KeywordIn,
    KeywordIf,
    KeywordThen,
    KeywordElse,
    KeywordEach,
    Identifier(String),
    StringLiteral(String),
    Number(String),
    Symbol(char),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MParseError {
    #[error("expression is empty")]
    Empty,
    #[error("unterminated string literal")]
    UnterminatedString,
    #[error("unterminated block comment")]
    UnterminatedBlockComment,
    #[error("unbalanced delimiter")]
    UnbalancedDelimiter,
    #[error("invalid let binding syntax")]
    InvalidLetBinding,
    #[error("missing 'in' clause in let expression")]
    MissingInClause,
}

impl From<&MToken> for MTokenDebug {
    fn from(token: &MToken) -> Self {
        match token {
            MToken::KeywordLet => MTokenDebug::KeywordLet,
            MToken::KeywordIn => MTokenDebug::KeywordIn,
            MToken::KeywordIf => MTokenDebug::KeywordIf,
            MToken::KeywordThen => MTokenDebug::KeywordThen,
            MToken::KeywordElse => MTokenDebug::KeywordElse,
            MToken::KeywordEach => MTokenDebug::KeywordEach,
            MToken::Identifier(v) => MTokenDebug::Identifier(v.clone()),
            MToken::StringLiteral(v) => MTokenDebug::StringLiteral(v.clone()),
            MToken::Number(v) => MTokenDebug::Number(v.clone()),
            MToken::Symbol(v) => MTokenDebug::Symbol(*v),
        }
    }
}

impl MModuleAst {
    /// Returns a minimal view of the root expression kind for tests and debugging.
    ///
    /// This keeps the AST opaque for production consumers while allowing
    /// tests to assert the expected structure.
    pub fn root_kind_for_testing(&self) -> MAstKind {
        fn access_chain_len(expr: &MExpr) -> usize {
            let mut n = 0usize;
            let mut cur = expr;
            while let MExpr::Access { base, .. } = cur {
                n += 1;
                cur = base;
            }
            n
        }

        match &self.root {
            MExpr::Let { bindings, .. } => MAstKind::Let {
                binding_count: bindings.len(),
            },
            MExpr::Record { fields } => MAstKind::Record {
                field_count: fields.len(),
            },
            MExpr::List { items } => MAstKind::List {
                item_count: items.len(),
            },
            MExpr::FunctionCall { name, args } => MAstKind::FunctionCall {
                name: name.clone(),
                arg_count: args.len(),
            },
            MExpr::FunctionLiteral { params, .. } => MAstKind::FunctionLiteral {
                param_count: params.len(),
            },
            MExpr::UnaryOp { .. } => MAstKind::UnaryOp,
            MExpr::BinaryOp { .. } => MAstKind::BinaryOp,
            MExpr::TypeAscription { .. } => MAstKind::TypeAscription,
            MExpr::TryOtherwise { .. } => MAstKind::TryOtherwise,
            MExpr::Primitive(_) => MAstKind::Primitive,
            MExpr::Ident { name } => MAstKind::Ident { name: name.clone() },
            MExpr::If { .. } => MAstKind::If,
            MExpr::Each { .. } => MAstKind::Each,
            MExpr::Access { kind, .. } => {
                let kind = match kind {
                    AccessKind::Field => MAstAccessKind::Field,
                    AccessKind::Item => MAstAccessKind::Item,
                };
                MAstKind::Access {
                    kind,
                    chain_len: access_chain_len(&self.root),
                }
            }
            MExpr::Opaque(tokens) => MAstKind::Opaque {
                token_count: tokens.len(),
            },
        }
    }

    pub(crate) fn root_expr(&self) -> &MExpr {
        &self.root
    }
}

impl MExpr {
    pub(crate) fn diff_label_hash(&self) -> u64 {
        use crate::hashing::XXH64_SEED;
        let mut h = xxhash_rust::xxh64::Xxh64::new(XXH64_SEED);
        match self {
            MExpr::Let { bindings, .. } => {
                0u8.hash(&mut h);
                bindings.len().hash(&mut h);
            }
            MExpr::Record { fields } => {
                1u8.hash(&mut h);
                fields.len().hash(&mut h);
            }
            MExpr::List { items } => {
                2u8.hash(&mut h);
                items.len().hash(&mut h);
            }
            MExpr::FunctionCall { name, args } => {
                3u8.hash(&mut h);
                name.to_ascii_lowercase().hash(&mut h);
                args.len().hash(&mut h);
            }
            MExpr::FunctionLiteral {
                params,
                return_type,
                ..
            } => {
                4u8.hash(&mut h);
                params.len().hash(&mut h);
                if let Some(rt) = return_type {
                    rt.name.to_ascii_lowercase().hash(&mut h);
                }
            }
            MExpr::UnaryOp { op, .. } => {
                5u8.hash(&mut h);
                op.hash(&mut h);
            }
            MExpr::BinaryOp { op, .. } => {
                6u8.hash(&mut h);
                op.hash(&mut h);
            }
            MExpr::TypeAscription { ty, .. } => {
                7u8.hash(&mut h);
                ty.name.to_ascii_lowercase().hash(&mut h);
            }
            MExpr::TryOtherwise { .. } => {
                8u8.hash(&mut h);
            }
            MExpr::Ident { name } => {
                9u8.hash(&mut h);
                name.hash(&mut h);
            }
            MExpr::If { .. } => {
                10u8.hash(&mut h);
            }
            MExpr::Each { .. } => {
                11u8.hash(&mut h);
            }
            MExpr::Access { kind, .. } => {
                12u8.hash(&mut h);
                kind.hash(&mut h);
            }
            MExpr::Primitive(p) => {
                13u8.hash(&mut h);
                p.hash(&mut h);
            }
            MExpr::Opaque(tokens) => {
                14u8.hash(&mut h);
                tokens.hash(&mut h);
            }
        }
        h.finish()
    }

    pub(crate) fn diff_children(&self) -> Vec<&MExpr> {
        let mut out = Vec::new();
        match self {
            MExpr::Let { bindings, body } => {
                for b in bindings {
                    out.push(b.value.as_ref());
                }
                out.push(body.as_ref());
            }
            MExpr::Record { fields } => {
                for f in fields {
                    out.push(&f.value);
                }
            }
            MExpr::List { items } => {
                for it in items {
                    out.push(it);
                }
            }
            MExpr::FunctionCall { args, .. } => {
                for a in args {
                    out.push(a);
                }
            }
            MExpr::FunctionLiteral { body, .. } => {
                out.push(body.as_ref());
            }
            MExpr::UnaryOp { expr, .. } => {
                out.push(expr.as_ref());
            }
            MExpr::BinaryOp { left, right, .. } => {
                out.push(left.as_ref());
                out.push(right.as_ref());
            }
            MExpr::TypeAscription { expr, .. } => {
                out.push(expr.as_ref());
            }
            MExpr::TryOtherwise { expr, otherwise } => {
                out.push(expr.as_ref());
                out.push(otherwise.as_ref());
            }
            MExpr::Ident { .. } => {}
            MExpr::If {
                cond,
                then_branch,
                else_branch,
            } => {
                out.push(cond.as_ref());
                out.push(then_branch.as_ref());
                out.push(else_branch.as_ref());
            }
            MExpr::Each { body } => {
                out.push(body.as_ref());
            }
            MExpr::Access { base, key, .. } => {
                out.push(base.as_ref());
                out.push(key.as_ref());
            }
            MExpr::Primitive(_) => {}
            MExpr::Opaque(_) => {}
        }
        out
    }
}

/// Tokenize an M expression for testing and diagnostics.
///
/// The returned tokens are a debug-friendly mirror of the internal lexer output
/// and are not part of the stable public API.
pub fn tokenize_for_testing(source: &str) -> Result<Vec<MTokenDebug>, MParseError> {
    tokenize(source).map(|tokens| tokens.iter().map(MTokenDebug::from).collect())
}

/// Parse a Power Query M expression into a minimal AST.
///
/// Supports top-level `let ... in ...` expressions, record and list literals,
/// qualified function calls, function literals, unary/binary ops, type
/// ascription, `try ... otherwise ...`, primitive literals, identifier
/// references, `if` expressions, `each` expressions, and access chains. Inputs
/// that do not match those forms are preserved as opaque token sequences. The
/// lexer recognizes `let`/`in`/`if`/`then`/`else`/`each`, quoted identifiers
/// (`#"Foo"`), and hash-prefixed literals like `#date`/`#datetime` as single
/// identifiers; other M constructs are parsed best-effort and may be treated as
/// generic tokens.
pub fn parse_m_expression(source: &str) -> Result<MModuleAst, MParseError> {
    let tokens = tokenize(source)?;
    if tokens.is_empty() {
        return Err(MParseError::Empty);
    }

    let root = parse_expression(&tokens)?;
    Ok(MModuleAst { root })
}

pub fn canonicalize_m_ast(ast: &mut MModuleAst) {
    canonicalize_expr(&mut ast.root);
}

pub fn ast_semantically_equal(a: &MModuleAst, b: &MModuleAst) -> bool {
    a == b
}

fn canonicalize_expr(expr: &mut MExpr) {
    match expr {
        MExpr::Let { bindings, body } => {
            for binding in bindings {
                canonicalize_expr(&mut binding.value);
            }
            canonicalize_expr(body);
        }
        MExpr::Record { fields } => {
            for field in fields.iter_mut() {
                canonicalize_expr(&mut field.value);
            }
            fields.sort_by(|a, b| a.name.cmp(&b.name));
        }
        MExpr::List { items } => {
            for item in items {
                canonicalize_expr(item);
            }
        }
        MExpr::FunctionCall { args, .. } => {
            for arg in args {
                canonicalize_expr(arg);
            }
        }
        MExpr::FunctionLiteral {
            params,
            return_type,
            body,
        } => {
            for param in params.iter_mut() {
                if let Some(ty) = param.ty.as_mut() {
                    ty.name = ty.name.to_ascii_lowercase();
                }
            }
            if let Some(ty) = return_type.as_mut() {
                ty.name = ty.name.to_ascii_lowercase();
            }
            canonicalize_expr(body);
        }
        MExpr::UnaryOp { expr, .. } => {
            canonicalize_expr(expr);
        }
        MExpr::BinaryOp { left, right, .. } => {
            canonicalize_expr(left);
            canonicalize_expr(right);
        }
        MExpr::TypeAscription { expr, ty } => {
            canonicalize_expr(expr);
            ty.name = ty.name.to_ascii_lowercase();
        }
        MExpr::TryOtherwise { expr, otherwise } => {
            canonicalize_expr(expr);
            canonicalize_expr(otherwise);
        }
        MExpr::Ident { .. } => {}
        MExpr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            canonicalize_expr(cond);
            canonicalize_expr(then_branch);
            canonicalize_expr(else_branch);
        }
        MExpr::Each { body } => {
            canonicalize_expr(body);
        }
        MExpr::Access { base, key, .. } => {
            canonicalize_expr(base);
            canonicalize_expr(key);
        }
        MExpr::Primitive(_) => {}
        MExpr::Opaque(tokens) => canonicalize_tokens(tokens),
    }
}

fn canonicalize_tokens(tokens: &mut Vec<MToken>) {
    for token in tokens.iter_mut() {
        let MToken::Identifier(ident) = token else {
            continue;
        };

        if ident.eq_ignore_ascii_case("true") {
            *ident = "true".to_string();
        } else if ident.eq_ignore_ascii_case("false") {
            *ident = "false".to_string();
        } else if ident.eq_ignore_ascii_case("null") {
            *ident = "null".to_string();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InfixSplit {
    Binary(MBinaryOp),
    Ascription,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SplitPoint {
    idx: usize,
    len: usize,
    prec: u8,
    kind: InfixSplit,
}

const PREC_OR: u8 = 10;
const PREC_AND: u8 = 20;
const PREC_CMP: u8 = 30;
const PREC_CONCAT: u8 = 40;
const PREC_ADD: u8 = 50;
const PREC_MUL: u8 = 60;

fn is_tail_keyword(tok: &MToken) -> bool {
    matches!(
        tok,
        MToken::KeywordIf | MToken::KeywordLet | MToken::KeywordEach
    )
}

fn is_try_head(tok: &MToken) -> bool {
    matches!(tok, MToken::Identifier(v) if v.eq_ignore_ascii_case("try"))
}

fn lambda_starts_at(tokens: &[MToken], i: usize) -> bool {
    if tokens.get(i) != Some(&MToken::Symbol('(')) {
        return false;
    }

    let mut depth = 0i32;
    let mut close: Option<usize> = None;
    for j in i..tokens.len() {
        match &tokens[j] {
            MToken::Symbol('(') => depth += 1,
            MToken::Symbol(')') => {
                depth -= 1;
                if depth == 0 {
                    close = Some(j);
                    break;
                }
            }
            _ => {}
        }
    }

    let Some(close) = close else {
        return false;
    };
    matches!(tokens.get(close + 1), Some(MToken::Symbol('=')))
        && matches!(tokens.get(close + 2), Some(MToken::Symbol('>')))
}

fn scan_cutoff_for_tail_expr(tokens: &[MToken]) -> usize {
    let mut depth = 0i32;

    for (i, tok) in tokens.iter().enumerate() {
        match tok {
            MToken::Symbol('(') | MToken::Symbol('[') | MToken::Symbol('{') => depth += 1,
            MToken::Symbol(')') | MToken::Symbol(']') | MToken::Symbol('}') => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            _ if depth == 0 && i > 0 => {
                if is_tail_keyword(tok) || is_try_head(tok) || lambda_starts_at(tokens, i) {
                    return i;
                }
            }
            _ => {}
        }
    }

    tokens.len()
}

fn consider_best(best: &mut Option<SplitPoint>, cand: SplitPoint, full_len: usize) {
    if cand.idx == 0 || cand.idx + cand.len >= full_len {
        return;
    }
    match best {
        None => *best = Some(cand),
        Some(b) => {
            if cand.prec < b.prec || (cand.prec == b.prec && cand.idx > b.idx) {
                *best = Some(cand);
            }
        }
    }
}

fn find_best_infix_split(tokens: &[MToken]) -> Option<SplitPoint> {
    let full_len = tokens.len();
    let cutoff = scan_cutoff_for_tail_expr(tokens);
    let tokens = &tokens[..cutoff];

    let mut best: Option<SplitPoint> = None;
    let mut depth = 0i32;

    let mut i = 0usize;
    while i < tokens.len() {
        match &tokens[i] {
            MToken::Symbol('(') | MToken::Symbol('[') | MToken::Symbol('{') => depth += 1,
            MToken::Symbol(')') | MToken::Symbol(']') | MToken::Symbol('}') => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            _ if depth == 0 => {
                if i + 1 < tokens.len() {
                    if tokens[i] == MToken::Symbol('<') && tokens[i + 1] == MToken::Symbol('>') {
                        consider_best(
                            &mut best,
                            SplitPoint {
                                idx: i,
                                len: 2,
                                prec: PREC_CMP,
                                kind: InfixSplit::Binary(MBinaryOp::Ne),
                            },
                            full_len,
                        );
                        i += 2;
                        continue;
                    }
                    if tokens[i] == MToken::Symbol('<') && tokens[i + 1] == MToken::Symbol('=') {
                        consider_best(
                            &mut best,
                            SplitPoint {
                                idx: i,
                                len: 2,
                                prec: PREC_CMP,
                                kind: InfixSplit::Binary(MBinaryOp::Le),
                            },
                            full_len,
                        );
                        i += 2;
                        continue;
                    }
                    if tokens[i] == MToken::Symbol('>') && tokens[i + 1] == MToken::Symbol('=') {
                        consider_best(
                            &mut best,
                            SplitPoint {
                                idx: i,
                                len: 2,
                                prec: PREC_CMP,
                                kind: InfixSplit::Binary(MBinaryOp::Ge),
                            },
                            full_len,
                        );
                        i += 2;
                        continue;
                    }
                }

                match &tokens[i] {
                    MToken::Symbol('+') => consider_best(
                        &mut best,
                        SplitPoint {
                            idx: i,
                            len: 1,
                            prec: PREC_ADD,
                            kind: InfixSplit::Binary(MBinaryOp::Add),
                        },
                        full_len,
                    ),
                    MToken::Symbol('-') => consider_best(
                        &mut best,
                        SplitPoint {
                            idx: i,
                            len: 1,
                            prec: PREC_ADD,
                            kind: InfixSplit::Binary(MBinaryOp::Sub),
                        },
                        full_len,
                    ),
                    MToken::Symbol('*') => consider_best(
                        &mut best,
                        SplitPoint {
                            idx: i,
                            len: 1,
                            prec: PREC_MUL,
                            kind: InfixSplit::Binary(MBinaryOp::Mul),
                        },
                        full_len,
                    ),
                    MToken::Symbol('/') => consider_best(
                        &mut best,
                        SplitPoint {
                            idx: i,
                            len: 1,
                            prec: PREC_MUL,
                            kind: InfixSplit::Binary(MBinaryOp::Div),
                        },
                        full_len,
                    ),
                    MToken::Symbol('&') => consider_best(
                        &mut best,
                        SplitPoint {
                            idx: i,
                            len: 1,
                            prec: PREC_CONCAT,
                            kind: InfixSplit::Binary(MBinaryOp::Concat),
                        },
                        full_len,
                    ),
                    MToken::Symbol('=') => consider_best(
                        &mut best,
                        SplitPoint {
                            idx: i,
                            len: 1,
                            prec: PREC_CMP,
                            kind: InfixSplit::Binary(MBinaryOp::Eq),
                        },
                        full_len,
                    ),
                    MToken::Symbol('<') => consider_best(
                        &mut best,
                        SplitPoint {
                            idx: i,
                            len: 1,
                            prec: PREC_CMP,
                            kind: InfixSplit::Binary(MBinaryOp::Lt),
                        },
                        full_len,
                    ),
                    MToken::Symbol('>') => consider_best(
                        &mut best,
                        SplitPoint {
                            idx: i,
                            len: 1,
                            prec: PREC_CMP,
                            kind: InfixSplit::Binary(MBinaryOp::Gt),
                        },
                        full_len,
                    ),
                    _ => {}
                }

                if let MToken::Identifier(v) = &tokens[i] {
                    if v.eq_ignore_ascii_case("and") {
                        consider_best(
                            &mut best,
                            SplitPoint {
                                idx: i,
                                len: 1,
                                prec: PREC_AND,
                                kind: InfixSplit::Binary(MBinaryOp::And),
                            },
                            full_len,
                        );
                    } else if v.eq_ignore_ascii_case("or") {
                        consider_best(
                            &mut best,
                            SplitPoint {
                                idx: i,
                                len: 1,
                                prec: PREC_OR,
                                kind: InfixSplit::Binary(MBinaryOp::Or),
                            },
                            full_len,
                        );
                    } else if v.eq_ignore_ascii_case("as") {
                        consider_best(
                            &mut best,
                            SplitPoint {
                                idx: i,
                                len: 1,
                                prec: PREC_CMP,
                                kind: InfixSplit::Ascription,
                            },
                            full_len,
                        );
                    }
                }
            }
            _ => {}
        }

        i += 1;
    }

    best
}

fn parse_tier2_ops(tokens: &[MToken]) -> Result<Option<MExpr>, MParseError> {
    if let Some(split) = find_best_infix_split(tokens) {
        let left_tokens = &tokens[..split.idx];
        let right_tokens = &tokens[split.idx + split.len..];

        if !left_tokens.is_empty() && !right_tokens.is_empty() {
            let left = parse_expression(left_tokens)?;
            match split.kind {
                InfixSplit::Binary(op) => {
                    let right = parse_expression(right_tokens)?;
                    return Ok(Some(MExpr::BinaryOp {
                        op,
                        left: Box::new(left),
                        right: Box::new(right),
                    }));
                }
                InfixSplit::Ascription => {
                    let ty = match parse_type_ref(right_tokens) {
                        Some(ty) => ty,
                        None => return Ok(None),
                    };
                    return Ok(Some(MExpr::TypeAscription {
                        expr: Box::new(left),
                        ty,
                    }));
                }
            }
        }
    }

    if tokens.len() >= 2 {
        if matches!(&tokens[0], MToken::Identifier(v) if v.eq_ignore_ascii_case("not")) {
            let inner = parse_expression(&tokens[1..])?;
            return Ok(Some(MExpr::UnaryOp {
                op: MUnaryOp::Not,
                expr: Box::new(inner),
            }));
        }

        if tokens[0] == MToken::Symbol('+') {
            let inner = parse_expression(&tokens[1..])?;
            return Ok(Some(MExpr::UnaryOp {
                op: MUnaryOp::Plus,
                expr: Box::new(inner),
            }));
        }

        if tokens[0] == MToken::Symbol('-') {
            if let Some(prim) = parse_primitive(tokens) {
                return Ok(Some(prim));
            }
            let inner = parse_expression(&tokens[1..])?;
            return Ok(Some(MExpr::UnaryOp {
                op: MUnaryOp::Minus,
                expr: Box::new(inner),
            }));
        }
    }

    Ok(None)
}

fn parse_expression(tokens: &[MToken]) -> Result<MExpr, MParseError> {
    if tokens.is_empty() {
        return Err(MParseError::Empty);
    }

    if let Some(let_ast) = parse_let(tokens)? {
        return Ok(let_ast);
    }

    if let Some(inner) = strip_wrapping_parens(tokens) {
        if !inner.is_empty() {
            return parse_expression(inner);
        }
    }

    if let Some(if_expr) = parse_if_then_else(tokens)? {
        return Ok(if_expr);
    }

    if let Some(each_expr) = parse_each_expr(tokens)? {
        return Ok(each_expr);
    }

    if let Some(try_expr) = parse_try_otherwise(tokens)? {
        return Ok(try_expr);
    }

    if let Some(fn_lit) = parse_function_literal(tokens)? {
        return Ok(fn_lit);
    }

    if let Some(op_expr) = parse_tier2_ops(tokens)? {
        return Ok(op_expr);
    }

    if let Some(rec) = parse_record_literal(tokens)? {
        return Ok(rec);
    }
    if let Some(list) = parse_list_literal(tokens)? {
        return Ok(list);
    }
    if let Some(access) = parse_access_chain(tokens)? {
        return Ok(access);
    }
    if let Some(call) = parse_function_call(tokens)? {
        return Ok(call);
    }
    if let Some(prim) = parse_primitive(tokens) {
        return Ok(prim);
    }
    if let Some(ident) = parse_ident_ref(tokens) {
        return Ok(ident);
    }

    Ok(MExpr::Opaque(tokens.to_vec()))
}

fn strip_wrapping_parens(tokens: &[MToken]) -> Option<&[MToken]> {
    if tokens.len() < 2 {
        return None;
    }
    if !matches!(tokens.first(), Some(MToken::Symbol('('))) {
        return None;
    }
    if !matches!(tokens.last(), Some(MToken::Symbol(')'))) {
        return None;
    }

    let mut depth = 0i32;
    for (i, tok) in tokens.iter().enumerate() {
        match tok {
            MToken::Symbol('(') => depth += 1,
            MToken::Symbol(')') => {
                depth -= 1;
                if depth == 0 && i != tokens.len() - 1 {
                    return None;
                }
            }
            _ => {}
        }
    }

    if depth != 0 {
        return None;
    }

    Some(&tokens[1..tokens.len() - 1])
}

fn is_wrapped_by(tokens: &[MToken], open: char, close: char) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if !matches!(tokens.first(), Some(MToken::Symbol(c)) if *c == open) {
        return false;
    }
    if !matches!(tokens.last(), Some(MToken::Symbol(c)) if *c == close) {
        return false;
    }

    let mut depth = 0i32;
    for (i, tok) in tokens.iter().enumerate() {
        match tok {
            MToken::Symbol('(') | MToken::Symbol('[') | MToken::Symbol('{') => depth += 1,
            MToken::Symbol(')') | MToken::Symbol(']') | MToken::Symbol('}') => {
                depth -= 1;
                if depth == 0 && i != tokens.len() - 1 {
                    return false;
                }
            }
            _ => {}
        }
    }

    depth == 0
}

fn split_top_level(tokens: &[MToken], delimiter: char) -> Vec<&[MToken]> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut depth = 0i32;
    let mut let_depth = 0i32;

    for (i, tok) in tokens.iter().enumerate() {
        match tok {
            MToken::Symbol('(') | MToken::Symbol('[') | MToken::Symbol('{') => depth += 1,
            MToken::Symbol(')') | MToken::Symbol(']') | MToken::Symbol('}') => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            MToken::KeywordLet => let_depth += 1,
            MToken::KeywordIn => {
                if let_depth > 0 {
                    let_depth -= 1;
                }
            }
            MToken::Symbol(c) if *c == delimiter && depth == 0 && let_depth == 0 => {
                out.push(&tokens[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }

    out.push(&tokens[start..]);
    out
}

fn token_as_name(tok: &MToken) -> Option<String> {
    match tok {
        MToken::Identifier(v) => Some(v.clone()),
        MToken::KeywordIf => Some("if".to_string()),
        MToken::KeywordThen => Some("then".to_string()),
        MToken::KeywordElse => Some("else".to_string()),
        MToken::KeywordEach => Some("each".to_string()),
        _ => None,
    }
}

fn parse_ident_ref(tokens: &[MToken]) -> Option<MExpr> {
    if tokens.len() != 1 {
        return None;
    }

    match &tokens[0] {
        MToken::Identifier(v) => Some(MExpr::Ident { name: v.clone() }),
        MToken::KeywordIf => Some(MExpr::Ident {
            name: "if".to_string(),
        }),
        MToken::KeywordThen => Some(MExpr::Ident {
            name: "then".to_string(),
        }),
        MToken::KeywordElse => Some(MExpr::Ident {
            name: "else".to_string(),
        }),
        MToken::KeywordEach => Some(MExpr::Ident {
            name: "each".to_string(),
        }),
        _ => None,
    }
}

fn parse_each_expr(tokens: &[MToken]) -> Result<Option<MExpr>, MParseError> {
    if !matches!(tokens.first(), Some(MToken::KeywordEach)) {
        return Ok(None);
    }
    if tokens.len() < 2 {
        return Ok(None);
    }

    let body = parse_expression(&tokens[1..])?;
    Ok(Some(MExpr::Each {
        body: Box::new(body),
    }))
}

fn parse_if_then_else(tokens: &[MToken]) -> Result<Option<MExpr>, MParseError> {
    if !matches!(tokens.first(), Some(MToken::KeywordIf)) {
        return Ok(None);
    }

    let mut depth = 0i32;
    let mut let_depth = 0i32;
    let mut if_depth = 0i32;
    let mut then_idx: Option<usize> = None;
    let mut else_idx: Option<usize> = None;

    for i in 1..tokens.len() {
        match &tokens[i] {
            MToken::Symbol('(') | MToken::Symbol('[') | MToken::Symbol('{') => depth += 1,
            MToken::Symbol(')') | MToken::Symbol(']') | MToken::Symbol('}') => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            MToken::KeywordLet => let_depth += 1,
            MToken::KeywordIn => {
                if let_depth > 0 {
                    let_depth -= 1;
                }
            }
            MToken::KeywordIf if depth == 0 && let_depth == 0 => {
                if_depth += 1;
            }
            MToken::KeywordThen
                if depth == 0 && let_depth == 0 && if_depth == 0 && then_idx.is_none() =>
            {
                then_idx = Some(i);
            }
            MToken::KeywordElse if depth == 0 && let_depth == 0 => {
                if if_depth > 0 {
                    if_depth -= 1;
                } else {
                    else_idx = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }

    let Some(then_idx) = then_idx else {
        return Ok(None);
    };
    let Some(else_idx) = else_idx else {
        return Ok(None);
    };

    if then_idx <= 1 {
        return Ok(None);
    }
    if else_idx <= then_idx + 1 {
        return Ok(None);
    }
    if else_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let cond = parse_expression(&tokens[1..then_idx])?;
    let then_branch = parse_expression(&tokens[then_idx + 1..else_idx])?;
    let else_branch = parse_expression(&tokens[else_idx + 1..])?;

    Ok(Some(MExpr::If {
        cond: Box::new(cond),
        then_branch: Box::new(then_branch),
        else_branch: Box::new(else_branch),
    }))
}

fn find_top_level_otherwise(tokens: &[MToken]) -> Option<usize> {
    let mut depth = 0i32;
    for (i, tok) in tokens.iter().enumerate() {
        match tok {
            MToken::Symbol('(') | MToken::Symbol('[') | MToken::Symbol('{') => depth += 1,
            MToken::Symbol(')') | MToken::Symbol(']') | MToken::Symbol('}') => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            MToken::Identifier(v) if depth == 0 && v.eq_ignore_ascii_case("otherwise") => {
                return Some(i);
            }
            _ => {}
        }
    }
    None
}

fn parse_try_otherwise(tokens: &[MToken]) -> Result<Option<MExpr>, MParseError> {
    if tokens.is_empty() {
        return Ok(None);
    }
    if !matches!(&tokens[0], MToken::Identifier(v) if v.eq_ignore_ascii_case("try")) {
        return Ok(None);
    }

    let otherwise_idx = match find_top_level_otherwise(&tokens[1..]) {
        Some(i) => i + 1,
        None => return Ok(None),
    };

    let expr_tokens = &tokens[1..otherwise_idx];
    let otherwise_tokens = &tokens[otherwise_idx + 1..];

    if expr_tokens.is_empty() || otherwise_tokens.is_empty() {
        return Ok(None);
    }

    let expr = parse_expression(expr_tokens)?;
    let otherwise = parse_expression(otherwise_tokens)?;

    Ok(Some(MExpr::TryOtherwise {
        expr: Box::new(expr),
        otherwise: Box::new(otherwise),
    }))
}

fn is_ident_token(tok: &MToken, s: &str) -> bool {
    matches!(tok, MToken::Identifier(v) if v.eq_ignore_ascii_case(s))
}

fn find_top_level_arrow(tokens: &[MToken]) -> Option<usize> {
    let mut depth = 0i32;
    for i in 0..tokens.len().saturating_sub(1) {
        match &tokens[i] {
            MToken::Symbol('(') | MToken::Symbol('[') | MToken::Symbol('{') => depth += 1,
            MToken::Symbol(')') | MToken::Symbol(']') | MToken::Symbol('}') => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            MToken::Symbol('=') if depth == 0 => {
                if matches!(tokens.get(i + 1), Some(MToken::Symbol('>'))) {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_param(tokens: &[MToken]) -> Option<MParam> {
    if tokens.is_empty() {
        return None;
    }

    if tokens.len() == 1 {
        let name = token_as_name(&tokens[0])?;
        return Some(MParam { name, ty: None });
    }

    for i in 1..tokens.len() {
        if is_ident_token(&tokens[i], "as") {
            let name = token_as_name(&tokens[0])?;
            let ty_tokens = &tokens[i + 1..];
            let ty = parse_type_ref(ty_tokens)?;
            return Some(MParam { name, ty: Some(ty) });
        }
    }

    None
}

fn parse_function_literal(tokens: &[MToken]) -> Result<Option<MExpr>, MParseError> {
    let arrow_idx = match find_top_level_arrow(tokens) {
        Some(i) => i,
        None => return Ok(None),
    };

    if tokens.first() != Some(&MToken::Symbol('(')) {
        return Ok(None);
    }
    if arrow_idx < 2 {
        return Ok(None);
    }

    let mut depth = 0i32;
    let mut close_paren: Option<usize> = None;
    for i in 0..arrow_idx {
        match &tokens[i] {
            MToken::Symbol('(') => depth += 1,
            MToken::Symbol(')') => {
                depth -= 1;
                if depth == 0 {
                    close_paren = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let close_paren = match close_paren {
        Some(i) => i,
        None => return Ok(None),
    };

    let params_tokens = &tokens[1..close_paren];
    let param_slices = if params_tokens.is_empty() {
        Vec::new()
    } else {
        split_top_level(params_tokens, ',')
    };

    let mut params = Vec::new();
    for slice in param_slices {
        if slice.is_empty() {
            return Ok(None);
        }
        let p = match parse_param(slice) {
            Some(p) => p,
            None => return Ok(None),
        };
        params.push(p);
    }

    let mut return_type: Option<MTypeRef> = None;
    let between = &tokens[close_paren + 1..arrow_idx];
    if !between.is_empty() {
        if between.len() >= 2 && is_ident_token(&between[0], "as") {
            let ty = match parse_type_ref(&between[1..]) {
                Some(ty) => ty,
                None => return Ok(None),
            };
            return_type = Some(ty);
        } else {
            return Ok(None);
        }
    }

    if tokens.len() <= arrow_idx + 1 {
        return Ok(None);
    }

    let body_tokens = &tokens[arrow_idx + 2..];
    if body_tokens.is_empty() {
        return Ok(None);
    }
    let body = parse_expression(body_tokens)?;

    Ok(Some(MExpr::FunctionLiteral {
        params,
        return_type,
        body: Box::new(body),
    }))
}

fn parse_access_chain(tokens: &[MToken]) -> Result<Option<MExpr>, MParseError> {
    if tokens.len() < 3 {
        return Ok(None);
    }

    let mut end = tokens.len();
    let mut segments: Vec<(AccessKind, &[MToken])> = Vec::new();

    loop {
        if end < 2 {
            break;
        }

        let (kind, open_ch, close_ch) = match &tokens[end - 1] {
            MToken::Symbol(']') => (AccessKind::Field, '[', ']'),
            MToken::Symbol('}') => (AccessKind::Item, '{', '}'),
            _ => break,
        };

        let mut depth = 0i32;
        let mut found_open: Option<usize> = None;

        for i in (0..end - 1).rev() {
            match &tokens[i] {
                MToken::Symbol(c) if *c == close_ch => depth += 1,
                MToken::Symbol(c) if *c == open_ch => {
                    if depth == 0 {
                        found_open = Some(i);
                        break;
                    } else {
                        depth -= 1;
                    }
                }
                _ => {}
            }
        }

        let Some(open_idx) = found_open else {
            return Ok(None);
        };

        if open_idx + 1 > end - 1 {
            return Ok(None);
        }

        let inner = &tokens[open_idx + 1..end - 1];
        segments.push((kind, inner));
        end = open_idx;
    }

    if segments.is_empty() {
        return Ok(None);
    }

    let base_tokens = &tokens[..end];
    if base_tokens.is_empty() {
        return Ok(None);
    }

    let mut expr = parse_expression(base_tokens)?;

    for (kind, key_tokens) in segments.into_iter().rev() {
        if key_tokens.is_empty() {
            return Ok(None);
        }
        let key = parse_expression(key_tokens)?;
        expr = MExpr::Access {
            base: Box::new(expr),
            kind,
            key: Box::new(key),
        };
    }

    Ok(Some(expr))
}

fn parse_record_literal(tokens: &[MToken]) -> Result<Option<MExpr>, MParseError> {
    if !is_wrapped_by(tokens, '[', ']') {
        return Ok(None);
    }

    let inner = &tokens[1..tokens.len() - 1];
    if inner.is_empty() {
        return Ok(Some(MExpr::Record { fields: Vec::new() }));
    }

    let parts = split_top_level(inner, ',');
    let mut fields = Vec::with_capacity(parts.len());

    for part in parts {
        if part.len() < 3 {
            return Ok(None);
        }

        let name = match &part[0] {
            MToken::StringLiteral(v) => v.clone(),
            tok => match token_as_name(tok) {
                Some(name) => name,
                None => return Ok(None),
            },
        };

        if !matches!(part[1], MToken::Symbol('=')) {
            return Ok(None);
        }

        let value_tokens = &part[2..];
        if value_tokens.is_empty() {
            return Ok(None);
        }

        let value_expr = parse_expression(value_tokens)?;
        fields.push(RecordField {
            name,
            value: Box::new(value_expr),
        });
    }

    Ok(Some(MExpr::Record { fields }))
}

fn parse_list_literal(tokens: &[MToken]) -> Result<Option<MExpr>, MParseError> {
    if !is_wrapped_by(tokens, '{', '}') {
        return Ok(None);
    }

    let inner = &tokens[1..tokens.len() - 1];
    if inner.is_empty() {
        return Ok(Some(MExpr::List { items: Vec::new() }));
    }

    let parts = split_top_level(inner, ',');
    let mut items = Vec::with_capacity(parts.len());
    for part in parts {
        if part.is_empty() {
            return Ok(None);
        }
        items.push(parse_expression(part)?);
    }

    Ok(Some(MExpr::List { items }))
}

fn parse_function_call(tokens: &[MToken]) -> Result<Option<MExpr>, MParseError> {
    if tokens.len() < 3 {
        return Ok(None);
    }
    if matches!(tokens.first(), Some(MToken::Symbol('('))) {
        return Ok(None);
    }

    let mut open_idx = None;
    let mut depth = 0i32;
    let mut let_depth = 0i32;

    for (i, tok) in tokens.iter().enumerate() {
        match tok {
            MToken::Symbol('(') => {
                if depth == 0 && let_depth == 0 {
                    open_idx = Some(i);
                    break;
                }
                depth += 1;
            }
            MToken::Symbol('[') | MToken::Symbol('{') => depth += 1,
            MToken::Symbol(')') | MToken::Symbol(']') | MToken::Symbol('}') => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            MToken::KeywordLet => let_depth += 1,
            MToken::KeywordIn => {
                if let_depth > 0 {
                    let_depth -= 1;
                }
            }
            _ => {}
        }
    }

    let open_idx = match open_idx {
        Some(i) if i > 0 => i,
        _ => return Ok(None),
    };

    if !matches!(tokens.last(), Some(MToken::Symbol(')'))) {
        return Ok(None);
    }

    let mut suffix_depth = 0i32;
    for (i, tok) in tokens[open_idx..].iter().enumerate() {
        match tok {
            MToken::Symbol('(') | MToken::Symbol('[') | MToken::Symbol('{') => suffix_depth += 1,
            MToken::Symbol(')') | MToken::Symbol(']') | MToken::Symbol('}') => {
                if suffix_depth > 0 {
                    suffix_depth -= 1;
                }
                if suffix_depth == 0 && i != tokens.len() - open_idx - 1 {
                    return Ok(None);
                }
            }
            _ => {}
        }
    }

    if suffix_depth != 0 {
        return Ok(None);
    }

    let name = match parse_qualified_name(&tokens[..open_idx]) {
        Some(v) => v,
        None => return Ok(None),
    };

    let arg_tokens = &tokens[open_idx + 1..tokens.len() - 1];
    let args = if arg_tokens.is_empty() {
        Vec::new()
    } else {
        let parts = split_top_level(arg_tokens, ',');
        let mut args = Vec::with_capacity(parts.len());
        for part in parts {
            if part.is_empty() {
                return Ok(None);
            }
            args.push(parse_expression(part)?);
        }
        args
    };

    Ok(Some(MExpr::FunctionCall { name, args }))
}

fn parse_qualified_name(tokens: &[MToken]) -> Option<String> {
    if tokens.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    let mut i = 0usize;

    match &tokens[i] {
        MToken::Identifier(v) => parts.push(v.clone()),
        _ => return None,
    }
    i += 1;

    while i < tokens.len() {
        match (&tokens[i], tokens.get(i + 1)) {
            (MToken::Symbol('.'), Some(MToken::Identifier(v))) => {
                parts.push(v.clone());
                i += 2;
            }
            _ => return None,
        }
    }

    Some(parts.join("."))
}

fn parse_type_ref(tokens: &[MToken]) -> Option<MTypeRef> {
    let name = parse_qualified_name(tokens)?;
    Some(MTypeRef {
        name: name.to_ascii_lowercase(),
    })
}

fn parse_primitive(tokens: &[MToken]) -> Option<MExpr> {
    if tokens.len() == 1 {
        match &tokens[0] {
            MToken::StringLiteral(v) => {
                return Some(MExpr::Primitive(MPrimitive::String(v.clone())));
            }
            MToken::Number(v) => {
                return Some(MExpr::Primitive(MPrimitive::Number(v.clone())));
            }
            MToken::Identifier(v) => {
                if v.eq_ignore_ascii_case("true") {
                    return Some(MExpr::Primitive(MPrimitive::Boolean(true)));
                }
                if v.eq_ignore_ascii_case("false") {
                    return Some(MExpr::Primitive(MPrimitive::Boolean(false)));
                }
                if v.eq_ignore_ascii_case("null") {
                    return Some(MExpr::Primitive(MPrimitive::Null));
                }
            }
            _ => {}
        }
    }

    if tokens.len() == 2 {
        if matches!(tokens[0], MToken::Symbol('-')) {
            if let MToken::Number(v) = &tokens[1] {
                return Some(MExpr::Primitive(MPrimitive::Number(format!("-{}", v))));
            }
        }
    }

    None
}

fn parse_let(tokens: &[MToken]) -> Result<Option<MExpr>, MParseError> {
    if tokens.is_empty() {
        return Err(MParseError::Empty);
    }
    if !matches!(tokens.first(), Some(MToken::KeywordLet)) {
        return Ok(None);
    }

    let mut idx = 1usize;
    let mut bindings = Vec::new();
    let mut found_in = false;

    while idx < tokens.len() {
        if matches!(tokens.get(idx), Some(MToken::KeywordIn)) {
            found_in = true;
            idx += 1;
            break;
        }

        let name = match tokens.get(idx) {
            Some(MToken::Identifier(name)) => name.clone(),
            _ => return Err(MParseError::InvalidLetBinding),
        };
        idx += 1;

        if !matches!(tokens.get(idx), Some(MToken::Symbol('='))) {
            return Err(MParseError::InvalidLetBinding);
        }
        idx += 1;

        let value_start = idx;
        let mut value_end = None;
        let mut depth = 0i32;
        let mut let_depth_in_value = 0i32;

        while idx < tokens.len() {
            match tokens.get(idx) {
                Some(MToken::KeywordLet) => {
                    let_depth_in_value += 1;
                }
                Some(MToken::KeywordIn) => {
                    if let_depth_in_value > 0 {
                        let_depth_in_value -= 1;
                    } else if depth == 0 {
                        value_end = Some(idx);
                        break;
                    }
                }
                Some(MToken::Symbol(c)) if is_open_delimiter(*c) => depth += 1,
                Some(MToken::Symbol(c)) if is_close_delimiter(*c) => {
                    if depth > 0 {
                        depth -= 1;
                    }
                }
                Some(MToken::Symbol(',')) if depth == 0 && let_depth_in_value == 0 => {
                    value_end = Some(idx);
                    break;
                }
                _ => {}
            }
            idx += 1;
        }

        let value_end = value_end.unwrap_or(idx);
        if value_end <= value_start {
            return Err(MParseError::InvalidLetBinding);
        }

        let value_tokens = &tokens[value_start..value_end];
        let value_expr = parse_expression(value_tokens)?;

        bindings.push(LetBinding {
            name,
            value: Box::new(value_expr),
        });

        idx = value_end;
        if matches!(tokens.get(idx), Some(MToken::Symbol(','))) {
            idx += 1;
        }
    }

    if !found_in {
        return Err(MParseError::MissingInClause);
    }
    if idx >= tokens.len() {
        return Err(MParseError::InvalidLetBinding);
    }

    let body_expr = parse_expression(&tokens[idx..])?;
    Ok(Some(MExpr::Let {
        bindings,
        body: Box::new(body_expr),
    }))
}

fn tokenize(source: &str) -> Result<Vec<MToken>, MParseError> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();
    let mut delimiters: Vec<char> = Vec::new();

    while let Some(ch) = chars.next() {
        if ch.is_whitespace() {
            continue;
        }

        if ch == '/' {
            if matches!(chars.peek(), Some('/')) {
                skip_line_comment(&mut chars);
                continue;
            }
            if matches!(chars.peek(), Some('*')) {
                chars.next();
                skip_block_comment(&mut chars)?;
                continue;
            }
        }

        if ch == '"' {
            let literal = parse_string(&mut chars)?;
            tokens.push(MToken::StringLiteral(literal));
            continue;
        }

        if ch == '#' {
            if matches!(chars.peek(), Some('"')) {
                chars.next();
                let ident = parse_string(&mut chars)?;
                tokens.push(MToken::Identifier(ident));
                continue;
            }
            if let Some(next) = chars.peek().copied()
                && is_identifier_start(next)
            {
                chars.next();
                let ident = parse_identifier(next, &mut chars);
                tokens.push(MToken::Identifier(format!("#{ident}")));
                continue;
            }
            tokens.push(MToken::Symbol('#'));
            continue;
        }

        if is_identifier_start(ch) {
            let ident = parse_identifier(ch, &mut chars);
            if ident.eq_ignore_ascii_case("let") {
                tokens.push(MToken::KeywordLet);
            } else if ident.eq_ignore_ascii_case("in") {
                tokens.push(MToken::KeywordIn);
            } else if ident.eq_ignore_ascii_case("if") {
                tokens.push(MToken::KeywordIf);
            } else if ident.eq_ignore_ascii_case("then") {
                tokens.push(MToken::KeywordThen);
            } else if ident.eq_ignore_ascii_case("else") {
                tokens.push(MToken::KeywordElse);
            } else if ident.eq_ignore_ascii_case("each") {
                tokens.push(MToken::KeywordEach);
            } else {
                tokens.push(MToken::Identifier(ident));
            }
            continue;
        }

        if ch.is_ascii_digit() {
            let number = parse_number(ch, &mut chars);
            tokens.push(MToken::Number(number));
            continue;
        }

        if is_open_delimiter(ch) {
            delimiters.push(ch);
        } else if is_close_delimiter(ch) {
            let Some(open) = delimiters.pop() else {
                return Err(MParseError::UnbalancedDelimiter);
            };
            if !delimiters_match(open, ch) {
                return Err(MParseError::UnbalancedDelimiter);
            }
        }

        tokens.push(MToken::Symbol(ch));
    }

    if !delimiters.is_empty() {
        return Err(MParseError::UnbalancedDelimiter);
    }

    Ok(tokens)
}

#[allow(clippy::while_let_on_iterator)]
fn skip_line_comment(chars: &mut Peekable<Chars<'_>>) {
    while let Some(ch) = chars.next() {
        if ch == '\n' {
            break;
        }
    }
}

#[allow(clippy::while_let_on_iterator)]
fn skip_block_comment(chars: &mut Peekable<Chars<'_>>) -> Result<(), MParseError> {
    while let Some(ch) = chars.next() {
        if ch == '*' && matches!(chars.peek(), Some('/')) {
            chars.next();
            return Ok(());
        }
    }

    Err(MParseError::UnterminatedBlockComment)
}

fn parse_string(chars: &mut Peekable<Chars<'_>>) -> Result<String, MParseError> {
    let mut buf = String::new();

    while let Some(ch) = chars.next() {
        if ch == '"' {
            if matches!(chars.peek(), Some('"')) {
                buf.push('"');
                chars.next();
                continue;
            }
            return Ok(buf);
        }

        buf.push(ch);
    }

    Err(MParseError::UnterminatedString)
}

fn parse_identifier(first: char, chars: &mut Peekable<Chars<'_>>) -> String {
    let mut ident = String::new();
    ident.push(first);

    while let Some(&next) = chars.peek() {
        if is_identifier_continue(next) {
            ident.push(next);
            chars.next();
        } else {
            break;
        }
    }

    ident
}

fn parse_number(first: char, chars: &mut Peekable<Chars<'_>>) -> String {
    let mut number = String::new();
    number.push(first);

    while let Some(&next) = chars.peek() {
        if next.is_ascii_digit() || next == '.' {
            number.push(next);
            chars.next();
        } else {
            break;
        }
    }

    number
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_identifier_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn is_open_delimiter(ch: char) -> bool {
    matches!(ch, '(' | '[' | '{')
}

fn is_close_delimiter(ch: char) -> bool {
    matches!(ch, ')' | ']' | '}')
}

fn delimiters_match(open: char, close: char) -> bool {
    matches!((open, close), ('(', ')') | ('[', ']') | ('{', '}'))
}
