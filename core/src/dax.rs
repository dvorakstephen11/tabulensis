use std::hash::{Hash, Hasher};

use xxhash_rust::xxh64::Xxh64;

use crate::hashing::XXH64_SEED;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DaxParseError {
    message: String,
}

impl DaxParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for DaxParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DaxParseError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BinaryOp {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Concat,
}

impl BinaryOp {
    fn precedence(self) -> u8 {
        match self {
            BinaryOp::Or => 1,
            BinaryOp::And => 2,
            BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => 3,
            BinaryOp::Concat => 4,
            BinaryOp::Add | BinaryOp::Sub => 5,
            BinaryOp::Mul | BinaryOp::Div => 6,
            BinaryOp::Pow => 7,
        }
    }

    fn right_assoc(self) -> bool {
        matches!(self, BinaryOp::Pow)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum UnaryOp {
    Pos,
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Expr {
    Number(u64),
    String(String),
    Boolean(bool),
    Identifier(String),
    BracketRef(String),
    TableColumnRef { table: String, column: String },
    Call { name: String, args: Vec<Expr> },
    Unary { op: UnaryOp, expr: Box<Expr> },
    Binary { op: BinaryOp, left: Box<Expr>, right: Box<Expr> },
    VarBlock { vars: Vec<(String, Expr)>, body: Box<Expr> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokenKind {
    Ident(String),
    BracketIdent(String),
    StringLiteral(String),
    Number(u64),
    Operator(BinaryOp),
    Plus,
    Minus,
    LParen,
    RParen,
    Comma,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    kind: TokenKind,
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
}

impl Lexer {
    fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += 1;
        Some(ch)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while matches!(self.peek(), Some(ch) if ch.is_whitespace()) {
                self.advance();
            }

            match (self.peek(), self.peek_next()) {
                (Some('/'), Some('/')) => {
                    self.advance();
                    self.advance();
                    while let Some(ch) = self.peek() {
                        self.advance();
                        if ch == '\n' || ch == '\r' {
                            break;
                        }
                    }
                }
                (Some('/'), Some('*')) => {
                    self.advance();
                    self.advance();
                    while let Some(ch) = self.advance() {
                        if ch == '*' && self.peek() == Some('/') {
                            self.advance();
                            break;
                        }
                    }
                }
                (Some('-'), Some('-')) => {
                    self.advance();
                    self.advance();
                    while let Some(ch) = self.peek() {
                        self.advance();
                        if ch == '\n' || ch == '\r' {
                            break;
                        }
                    }
                }
                _ => break,
            }
        }
    }

    fn next_token(&mut self) -> Result<Token, DaxParseError> {
        self.skip_whitespace_and_comments();
        let Some(ch) = self.peek() else {
            return Ok(Token { kind: TokenKind::End });
        };

        match ch {
            '(' => {
                self.advance();
                Ok(Token { kind: TokenKind::LParen })
            }
            ')' => {
                self.advance();
                Ok(Token { kind: TokenKind::RParen })
            }
            ',' => {
                self.advance();
                Ok(Token { kind: TokenKind::Comma })
            }
            '[' => self.read_bracket_ident(),
            '\'' => self.read_quoted_ident(),
            '"' => self.read_string_literal(),
            '+' => {
                self.advance();
                Ok(Token { kind: TokenKind::Plus })
            }
            '-' => {
                self.advance();
                Ok(Token { kind: TokenKind::Minus })
            }
            '*' => {
                self.advance();
                Ok(Token { kind: TokenKind::Operator(BinaryOp::Mul) })
            }
            '/' => {
                self.advance();
                Ok(Token { kind: TokenKind::Operator(BinaryOp::Div) })
            }
            '^' => {
                self.advance();
                Ok(Token { kind: TokenKind::Operator(BinaryOp::Pow) })
            }
            '&' => {
                self.advance();
                if self.peek() == Some('&') {
                    self.advance();
                    Ok(Token { kind: TokenKind::Operator(BinaryOp::And) })
                } else {
                    Ok(Token { kind: TokenKind::Operator(BinaryOp::Concat) })
                }
            }
            '|' => {
                self.advance();
                if self.peek() == Some('|') {
                    self.advance();
                    Ok(Token { kind: TokenKind::Operator(BinaryOp::Or) })
                } else {
                    Err(DaxParseError::new("unexpected '|'"))
                }
            }
            '=' => {
                self.advance();
                Ok(Token { kind: TokenKind::Operator(BinaryOp::Eq) })
            }
            '<' => {
                self.advance();
                match self.peek() {
                    Some('=') => {
                        self.advance();
                        Ok(Token { kind: TokenKind::Operator(BinaryOp::Le) })
                    }
                    Some('>') => {
                        self.advance();
                        Ok(Token { kind: TokenKind::Operator(BinaryOp::Ne) })
                    }
                    _ => Ok(Token { kind: TokenKind::Operator(BinaryOp::Lt) }),
                }
            }
            '>' => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token { kind: TokenKind::Operator(BinaryOp::Ge) })
                } else {
                    Ok(Token { kind: TokenKind::Operator(BinaryOp::Gt) })
                }
            }
            _ => {
                if ch.is_ascii_digit() || (ch == '.' && self.peek_next().map(|n| n.is_ascii_digit()).unwrap_or(false)) {
                    self.read_number()
                } else if ch.is_ascii_alphabetic() || ch == '_' {
                    self.read_ident()
                } else {
                    Err(DaxParseError::new(format!("unexpected character '{}'", ch)))
                }
            }
        }
    }

    fn read_ident(&mut self) -> Result<Token, DaxParseError> {
        let mut buf = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' {
                buf.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        Ok(Token {
            kind: TokenKind::Ident(normalize_ident(&buf)),
        })
    }

    fn read_number(&mut self) -> Result<Token, DaxParseError> {
        let mut buf = String::new();
        let mut seen_dot = false;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                buf.push(ch);
                self.advance();
            } else if ch == '.' && !seen_dot {
                seen_dot = true;
                buf.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        if let Some(ch) = self.peek() {
            if ch == 'e' || ch == 'E' {
                buf.push(ch);
                self.advance();
                if let Some(sign) = self.peek() {
                    if sign == '+' || sign == '-' {
                        buf.push(sign);
                        self.advance();
                    }
                }
                while let Some(d) = self.peek() {
                    if d.is_ascii_digit() {
                        buf.push(d);
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }

        let num = buf.parse::<f64>().map_err(|_| {
            DaxParseError::new(format!("invalid number literal '{}'", buf))
        })?;
        Ok(Token {
            kind: TokenKind::Number(num.to_bits()),
        })
    }

    fn read_string_literal(&mut self) -> Result<Token, DaxParseError> {
        let mut buf = String::new();
        self.advance();
        while let Some(ch) = self.peek() {
            self.advance();
            if ch == '"' {
                if self.peek() == Some('"') {
                    self.advance();
                    buf.push('"');
                    continue;
                }
                return Ok(Token {
                    kind: TokenKind::StringLiteral(buf),
                });
            }
            buf.push(ch);
        }
        Err(DaxParseError::new("unterminated string literal"))
    }

    fn read_bracket_ident(&mut self) -> Result<Token, DaxParseError> {
        let mut buf = String::new();
        self.advance();
        while let Some(ch) = self.peek() {
            self.advance();
            if ch == ']' {
                if self.peek() == Some(']') {
                    self.advance();
                    buf.push(']');
                    continue;
                }
                return Ok(Token {
                    kind: TokenKind::BracketIdent(normalize_ident(&buf)),
                });
            }
            buf.push(ch);
        }
        Err(DaxParseError::new("unterminated bracket identifier"))
    }

    fn read_quoted_ident(&mut self) -> Result<Token, DaxParseError> {
        let mut buf = String::new();
        self.advance();
        while let Some(ch) = self.peek() {
            self.advance();
            if ch == '\'' {
                if self.peek() == Some('\'') {
                    self.advance();
                    buf.push('\'');
                    continue;
                }
                return Ok(Token {
                    kind: TokenKind::Ident(normalize_ident(&buf)),
                });
            }
            buf.push(ch);
        }
        Err(DaxParseError::new("unterminated quoted identifier"))
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn next(&mut self) -> &TokenKind {
        let tok = &self.tokens[self.pos].kind;
        self.pos = self.pos.saturating_add(1);
        tok
    }

    fn consume(&mut self, expected: &TokenKind) -> Result<(), DaxParseError> {
        if self.peek() == expected {
            self.next();
            Ok(())
        } else {
            Err(DaxParseError::new(format!(
                "expected {:?}, got {:?}",
                expected,
                self.peek()
            )))
        }
    }

    fn parse(&mut self) -> Result<Expr, DaxParseError> {
        let expr = self.parse_expr_bp(0)?;
        if !matches!(self.peek(), TokenKind::End) {
            return Err(DaxParseError::new("unexpected trailing tokens"));
        }
        Ok(expr)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr, DaxParseError> {
        let mut lhs = self.parse_prefix()?;
        loop {
            let op = match self.peek() {
                TokenKind::Operator(op) => *op,
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };

            let prec = op.precedence();
            if prec < min_bp {
                break;
            }
            let next_min = if op.right_assoc() { prec } else { prec + 1 };
            self.next();
            let rhs = self.parse_expr_bp(next_min)?;
            lhs = Expr::Binary {
                op,
                left: Box::new(lhs),
                right: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr, DaxParseError> {
        let token = self.next().clone();
        match token {
            TokenKind::Ident(name) if name == "var" => self.parse_var_block(),
            TokenKind::Ident(name) if name == "not" => {
                let expr = self.parse_expr_bp(8)?;
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                })
            }
            TokenKind::Ident(name) if name == "true" => Ok(Expr::Boolean(true)),
            TokenKind::Ident(name) if name == "false" => Ok(Expr::Boolean(false)),
            TokenKind::Ident(name) => {
                if matches!(self.peek(), TokenKind::LParen) {
                    self.next();
                    let args = self.parse_call_args()?;
                    Ok(Expr::Call { name, args })
                } else if let TokenKind::BracketIdent(column) = self.peek().clone() {
                    self.next();
                    Ok(Expr::TableColumnRef {
                        table: name,
                        column,
                    })
                } else {
                    Ok(Expr::Identifier(name))
                }
            }
            TokenKind::BracketIdent(name) => Ok(Expr::BracketRef(name)),
            TokenKind::StringLiteral(s) => Ok(Expr::String(s)),
            TokenKind::Number(n) => Ok(Expr::Number(n)),
            TokenKind::Plus => {
                let expr = self.parse_expr_bp(8)?;
                Ok(Expr::Unary {
                    op: UnaryOp::Pos,
                    expr: Box::new(expr),
                })
            }
            TokenKind::Minus => {
                let expr = self.parse_expr_bp(8)?;
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                })
            }
            TokenKind::LParen => {
                let expr = self.parse_expr_bp(0)?;
                self.consume(&TokenKind::RParen)?;
                Ok(expr)
            }
            other => Err(DaxParseError::new(format!("unexpected token {:?}", other))),
        }
    }

    fn parse_call_args(&mut self) -> Result<Vec<Expr>, DaxParseError> {
        let mut args = Vec::new();
        if matches!(self.peek(), TokenKind::RParen) {
            self.next();
            return Ok(args);
        }
        loop {
            let expr = self.parse_expr_bp(0)?;
            args.push(expr);
            match self.peek() {
                TokenKind::Comma => {
                    self.next();
                }
                TokenKind::RParen => {
                    self.next();
                    break;
                }
                _ => {
                    return Err(DaxParseError::new("expected ',' or ')' in call"));
                }
            }
        }
        Ok(args)
    }

    fn parse_var_block(&mut self) -> Result<Expr, DaxParseError> {
        let mut vars = Vec::new();
        loop {
            let name = match self.next() {
                TokenKind::Ident(name) => name.clone(),
                other => {
                    return Err(DaxParseError::new(format!(
                        "expected identifier after VAR, got {:?}",
                        other
                    )));
                }
            };
            self.consume(&TokenKind::Operator(BinaryOp::Eq))?;
            let expr = self.parse_expr_bp(0)?;
            vars.push((name, expr));

            match self.peek() {
                TokenKind::Ident(next) if next == "var" => {
                    self.next();
                    continue;
                }
                TokenKind::Ident(next) if next == "return" => {
                    self.next();
                    break;
                }
                _ => {
                    return Err(DaxParseError::new("expected VAR or RETURN"));
                }
            }
        }
        let body = self.parse_expr_bp(0)?;
        Ok(Expr::VarBlock {
            vars,
            body: Box::new(body),
        })
    }
}

fn normalize_ident(s: &str) -> String {
    s.to_lowercase()
}

pub(crate) fn semantic_hash(expr: &str) -> Result<u64, DaxParseError> {
    let mut lexer = Lexer::new(expr);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token()?;
        let end = matches!(token.kind, TokenKind::End);
        tokens.push(token);
        if end {
            break;
        }
    }

    let mut parser = Parser::new(tokens);
    let expr = parser.parse()?;
    let mut h = Xxh64::new(XXH64_SEED);
    expr.hash(&mut h);
    Ok(h.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(expr: &str) -> Result<u64, DaxParseError> {
        semantic_hash(expr)
    }

    #[test]
    fn dax_semantic_hash_ignores_whitespace_and_case() {
        let a = "SUM ( Sales[Amount] )";
        let b = "sum(sales[amount])";
        assert_eq!(hash(a).unwrap(), hash(b).unwrap());
    }

    #[test]
    fn dax_semantic_hash_var_block() {
        let a = "VAR x = 1 RETURN x + 2";
        let b = "var X=1 return X+2";
        assert_eq!(hash(a).unwrap(), hash(b).unwrap());
    }

    #[test]
    fn dax_semantic_hash_detects_semantic_change() {
        let a = "SUM(Sales[Amount])";
        let b = "SUM(Sales[Net])";
        assert_ne!(hash(a).unwrap(), hash(b).unwrap());
    }

    #[test]
    fn dax_semantic_hash_handles_binary_plus_minus() {
        let a = "1+2-3";
        let b = "1 + 2 - 3";
        assert_eq!(hash(a).unwrap(), hash(b).unwrap());
    }

    #[test]
    fn dax_semantic_hash_handles_comments() {
        let a = "SUM(Sales[Amount]) // comment";
        let b = "SUM(Sales[Amount])";
        assert_eq!(hash(a).unwrap(), hash(b).unwrap());
    }
}
