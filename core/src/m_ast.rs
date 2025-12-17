use std::iter::Peekable;
use std::str::Chars;

use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MModuleAst {
    root: MExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MAstKind {
    Let { binding_count: usize },
    Record { field_count: usize },
    List { item_count: usize },
    FunctionCall { name: String, arg_count: usize },
    Primitive,
    Opaque { token_count: usize },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MExpr {
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
    Primitive(MPrimitive),
    Opaque(Vec<MToken>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct LetBinding {
    name: String,
    value: Box<MExpr>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct RecordField {
    name: String,
    value: Box<MExpr>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MPrimitive {
    String(String),
    Number(String),
    Boolean(bool),
    Null,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MToken {
    KeywordLet,
    KeywordIn,
    Identifier(String),
    StringLiteral(String),
    Number(String),
    Symbol(char),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MTokenDebug {
    KeywordLet,
    KeywordIn,
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
            MExpr::Primitive(_) => MAstKind::Primitive,
            MExpr::Opaque(tokens) => MAstKind::Opaque {
                token_count: tokens.len(),
            },
        }
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
/// qualified function calls, and primitive literals. Inputs that do not match
/// those forms are preserved as opaque token sequences. The lexer recognizes
/// `let`/`in`, quoted identifiers (`#"Foo"`), and hash-prefixed literals like
/// `#date`/`#datetime` as single identifiers; other M constructs are parsed
/// best-effort and may be treated as generic tokens.
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

    if let Some(rec) = parse_record_literal(tokens)? {
        return Ok(rec);
    }
    if let Some(list) = parse_list_literal(tokens)? {
        return Ok(list);
    }
    if let Some(call) = parse_function_call(tokens)? {
        return Ok(call);
    }
    if let Some(prim) = parse_primitive(tokens) {
        return Ok(prim);
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

fn parse_record_literal(tokens: &[MToken]) -> Result<Option<MExpr>, MParseError> {
    if tokens.len() < 2 {
        return Ok(None);
    }
    if !matches!(tokens.first(), Some(MToken::Symbol('['))) {
        return Ok(None);
    }
    if !matches!(tokens.last(), Some(MToken::Symbol(']'))) {
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
            MToken::Identifier(v) => v.clone(),
            MToken::StringLiteral(v) => v.clone(),
            _ => return Ok(None),
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
    if tokens.len() < 2 {
        return Ok(None);
    }
    if !matches!(tokens.first(), Some(MToken::Symbol('{'))) {
        return Ok(None);
    }
    if !matches!(tokens.last(), Some(MToken::Symbol('}'))) {
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
