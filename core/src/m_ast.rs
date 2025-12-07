use std::iter::Peekable;
use std::str::Chars;

use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MModuleAst {
    root: MExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MExpr {
    Let {
        bindings: Vec<LetBinding>,
        body: Box<MExpr>,
    },
    Sequence(Vec<MToken>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LetBinding {
    name: String,
    value: Box<MExpr>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MToken {
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
        MExpr::Sequence(tokens) => canonicalize_tokens(tokens),
    }
}

fn canonicalize_tokens(tokens: &mut Vec<MToken>) {
    // Tokens are already whitespace/comment free; no additional normalization needed yet.
    let _ = tokens;
}

fn parse_expression(tokens: &[MToken]) -> Result<MExpr, MParseError> {
    if let Some(let_ast) = parse_let(tokens)? {
        return Ok(let_ast);
    }

    Ok(MExpr::Sequence(tokens.to_vec()))
}

fn parse_let(tokens: &[MToken]) -> Result<Option<MExpr>, MParseError> {
    if !matches!(tokens.first(), Some(MToken::KeywordLet)) {
        return Ok(None);
    }

    let mut idx = 1usize;
    let mut bindings = Vec::new();
    let mut found_in = false;

    while idx < tokens.len() {
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
        let mut depth = 0i32;
        let mut value_end: Option<usize> = None;

        while idx < tokens.len() {
            match &tokens[idx] {
                MToken::Symbol(c) if *c == '(' || *c == '[' || *c == '{' => depth += 1,
                MToken::Symbol(c) if *c == ')' || *c == ']' || *c == '}' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                }
                MToken::Symbol(',') if depth == 0 => {
                    value_end = Some(idx);
                    idx += 1;
                    break;
                }
                MToken::KeywordIn if depth == 0 => {
                    value_end = Some(idx);
                    found_in = true;
                    break;
                }
                _ => {}
            }

            idx += 1;
        }

        let end = value_end.ok_or(MParseError::MissingInClause)?;
        if end <= value_start {
            return Err(MParseError::InvalidLetBinding);
        }

        let value_expr = parse_expression(&tokens[value_start..end])?;
        bindings.push(LetBinding {
            name,
            value: Box::new(value_expr),
        });

        if found_in {
            idx = end + 1; // skip the 'in' token
            break;
        }
    }

    if !found_in {
        return Err(MParseError::MissingInClause);
    }

    if idx > tokens.len() {
        return Err(MParseError::InvalidLetBinding);
    }

    let body_tokens = &tokens[idx..];
    if body_tokens.is_empty() {
        return Err(MParseError::InvalidLetBinding);
    }
    let body = parse_expression(body_tokens)?;

    Ok(Some(MExpr::Let {
        bindings,
        body: Box::new(body),
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
