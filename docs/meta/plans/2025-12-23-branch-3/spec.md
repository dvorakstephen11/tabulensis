## Branch 3 target outcome

Branch 3 (“`2025-12-24-m-parser-tier2-ops-fn-types`”) is explicitly about turning the remaining Tier‑2 “token soup” into structured nodes so semantic diffs can understand *real* expression meaning: function literals, unary ops, binary ops with precedence, type ascription, and (optionally) `try … otherwise …`. 

Right now, the parser is a best‑effort decision tree that falls back to `MExpr::Opaque(tokens)` for anything outside Tier‑1 and the earlier literal/call forms (record/list/call/primitive/etc.).
Your coverage audit test calls out Tier‑2 examples that are **still opaque today**: `(x) => x`, `1 + 2`, `not true`, `x as number`.

The “exquisite” part of this branch is not just “add a few parse cases”; it’s making operator parsing *systematic* (precedence + associativity + nesting), so that canonicalization can reliably mask formatting-only variance.

---

## High-level strategy

### 1) Extend the internal AST (still opaque externally)

Add Tier‑2 nodes to the internal `MExpr` enum (and corresponding minimal shapes to `MAstKind` for tests/debug). Keep `MExpr` private; tests keep using `root_kind_for_testing()`.

### 2) Insert a “Tier‑2 precedence layer” into `parse_expression`

Instead of more ad-hoc “try parse X” cases, implement a precedence-aware parse step that:

* identifies top-level operators (respecting parentheses/brackets/braces),
* respects constructs that “consume the rest of the slice” (e.g., `if`, `let`, `each`, `try`, and function literals),
* builds a stable operator tree (associativity preserved),
* then falls back to existing Tier‑1/literal parsing.

This satisfies the sprint note that the tier‑2 step should become a “proper precedence parser (Pratt or precedence-climbing)” to avoid fragility with nested constructs. 

### 3) Canonicalization must traverse and normalize new nodes

Canonicalization currently recurses over known nodes and sorts record fields; opaque-token canonicalization normalizes `true/false/null` identifiers.
We’ll extend canonicalization to:

* recurse into operands/bodies,
* normalize type identifiers (`Number` vs `number`) so casts don’t churn hashes,
* ensure parentheses-only variance yields identical trees (by construction via parsing + existing `strip_wrapping_parens`).

### 4) Update tests: coverage audit + targeted tier‑2 unit tests

The plan explicitly calls for:

* precedence tests like `1 + 2 * 3` parsing as `+(1, *(2,3))`,
* semantic equality tests where formatting differs. 
  And your existing audit test must stop expecting tier‑2 cases to remain opaque.

---

## Step-by-step implementation plan

### Step 0 — Safety rails: keep current behavior as fallback

The existing `parse_expression()` tree ends with `Opaque(tokens.to_vec())`. 
We’ll preserve that as the final fallback so we don’t regress parsing success rates while we introduce Tier‑2 parsing.

---

## Step 1 — Extend `MExpr` and expose new minimal `MAstKind` shapes

### 1.1 Add Tier‑2 enums/structs (internal)

Add:

* `MUnaryOp` (`Not`, `Plus`, `Minus`)
* `MBinaryOp` (`Add`, `Sub`, `Mul`, `Div`, `Concat`, `Eq`, `Ne`, `Lt`, `Le`, `Gt`, `Ge`, `And`, `Or`)
* `MTypeRef` as minimal type grammar (for now: qualified name string)
* `MParam` for function literal params (name + optional type)
* `MExpr::FunctionLiteral`, `MExpr::UnaryOp`, `MExpr::BinaryOp`, `MExpr::TypeAscription`, `MExpr::TryOtherwise`

This directly matches Branch 3 scope. 

#### Replace: the current `MExpr` enum

From `core/src/m_ast.rs` (current excerpt): 

```rust
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
```

With:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MUnaryOp {
    Not,
    Plus,
    Minus,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MBinaryOp {
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
struct MTypeRef {
    name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct MParam {
    name: String,
    ty: Option<MTypeRef>,
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
```

### 1.2 Extend `MAstKind` and `root_kind_for_testing()`

Right now, tests can only assert `Let/Record/List/FunctionCall/Primitive/Ident/If/Each/Access/Opaque`.
Branch 3 tests need to observe:

* function literals,
* unary op,
* binary op,
* type ascription,
* try/otherwise (if implemented).

Add minimal variants:

* `FunctionLiteral { param_count: usize }`
* `UnaryOp`
* `BinaryOp`
* `TypeAscription`
* `TryOtherwise`

(Keep them minimal; you can always add op details later.)

#### Replace: the current `MAstKind` enum

From `core/src/m_ast.rs` (excerpt): 

```rust
pub enum MAstKind {
    Let { binding_count: usize },
    Record { field_count: usize },
    List { item_count: usize },
    FunctionCall { name: String, arg_count: usize },
    Primitive,
    Ident { name: String },
    If,
    Each,
    Access { kind: MAstAccessKind, chain_len: usize },
    Opaque { token_count: usize },
}
```

With:

```rust
pub enum MAstKind {
    Let { binding_count: usize },
    Record { field_count: usize },
    List { item_count: usize },
    FunctionCall { name: String, arg_count: usize },

    FunctionLiteral { param_count: usize },
    UnaryOp,
    BinaryOp,
    TypeAscription,
    TryOtherwise,

    Primitive,
    Ident { name: String },
    If,
    Each,
    Access { kind: MAstAccessKind, chain_len: usize },
    Opaque { token_count: usize },
}
```

#### Replace: the `root_kind_for_testing()` match arms

From the existing match (excerpt): 

```rust
match &self.root {
    MExpr::Let { bindings, .. } => MAstKind::Let { binding_count: bindings.len() },
    MExpr::Record { fields } => MAstKind::Record { field_count: fields.len() },
    MExpr::List { items } => MAstKind::List { item_count: items.len() },
    MExpr::FunctionCall { name, args } => MAstKind::FunctionCall {
        name: name.clone(),
        arg_count: args.len(),
    },
    MExpr::Primitive(_) => MAstKind::Primitive,
    MExpr::Ident { name } => MAstKind::Ident { name: name.clone() },
    MExpr::If { .. } => MAstKind::If,
    MExpr::Each { .. } => MAstKind::Each,
    MExpr::Access { kind, .. } => { /* ... */ }
    MExpr::Opaque(tokens) => MAstKind::Opaque { token_count: tokens.len() },
}
```

With:

```rust
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
```

---

## Step 2 — Canonicalization must include Tier‑2 nodes

Canonicalization currently:

* recurses into expressions,
* sorts record fields by name,
* canonicalizes `Opaque` token identifiers to lowercase for `true/false/null`.

We need to add recursion for the new nodes and normalize type names.

### 2.1 Normalize `MTypeRef` names

Minimal type grammar is “identifiers like `number`, `text`”. 
To avoid churn, store **canonical** type names (lowercase ASCII) at parse time, and keep canonicalize as a safety net.

### 2.2 Extend `canonicalize_expr`

#### Replace: current `canonicalize_expr` match

From `core/src/m_ast.rs` (excerpt): 

```rust
fn canonicalize_expr(expr: &mut MExpr) {
    match expr {
        MExpr::Let { bindings, body } => { /* ... */ }
        MExpr::Record { fields } => { /* ... */ }
        MExpr::List { items } => { /* ... */ }
        MExpr::FunctionCall { args, .. } => { /* ... */ }
        MExpr::Ident { .. } => {}
        MExpr::If { cond, then_branch, else_branch } => { /* ... */ }
        MExpr::Each { body } => { /* ... */ }
        MExpr::Access { base, key, .. } => { /* ... */ }
        MExpr::Primitive(_) => {}
        MExpr::Opaque(tokens) => canonicalize_tokens(tokens),
    }
}
```

With:

```rust
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

        MExpr::FunctionLiteral { params, return_type, body } => {
            for p in params.iter_mut() {
                if let Some(ty) = p.ty.as_mut() {
                    ty.name = ty.name.to_ascii_lowercase();
                }
            }
            if let Some(ret) = return_type.as_mut() {
                ret.name = ret.name.to_ascii_lowercase();
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
        MExpr::If { cond, then_branch, else_branch } => {
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
```

---

## Step 3 — Harden record/list literal matching (critical for `&` and other ops)

This is an important “silent correctness prerequisite”.

Right now `parse_record_literal` checks only `(first == '[' && last == ']')` and then assumes it’s a single record literal. 
Once you support operators like `&`, an expression like:

* `[A=1] & [B=2]`

**must not** be mistaken for a single record literal that spans from the first `[` to the final `]`. (This kind of bug will create very wrong trees and very wrong hashes.)

You already solved this exact problem for parenthesis stripping (`strip_wrapping_parens`) by verifying that the parens close only at the end of the slice. 
Do the same for `[`…`]` and `{`…`}` before accepting record/list literals.

### 3.1 Add a shared helper

Add a helper modeled after `strip_wrapping_parens`, generalized to “wrapping delimiter pair encloses entire slice”.

Add new code (no replacement; this is additive):

```rust
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
```

### 3.2 Update record/list literal matchers

Change the initial checks inside `parse_record_literal` and `parse_list_literal` from “first/last token” to `is_wrapped_by(tokens, '[', ']')` / `is_wrapped_by(tokens, '{', '}')`.

This prevents accidental spanning across operators, which becomes much more common once `&`, comparisons, etc. exist.

---

## Step 4 — Minimal type grammar (`<type>`)

Branch says: “Minimal type grammar to start (identifiers like `number`, `text`, etc.).” 

Implement:

* `parse_type_ref(tokens: &[MToken]) -> Option<MTypeRef>`

  * Accept qualified name tokens: `Identifier ('.' Identifier)*`
  * Lowercase at parse time.

Add new code:

```rust
fn parse_type_ref(tokens: &[MToken]) -> Option<MTypeRef> {
    let name = parse_qualified_name(tokens)?;
    Some(MTypeRef {
        name: name.to_ascii_lowercase(),
    })
}
```

(Reuse `parse_qualified_name` you already have for function calls; it’s the same token shape. )

---

## Step 5 — Function literals `(x, y) => expr`

Branch scope requires at least `(x) => x` and ideally multi-arg. 

### 5.1 Parsing approach

Recognize at top-level:

* `(<params>) => <body>`
* optionally support return type:

  * `(<params>) as <type> => <body>` (very common in M)

Parameters:

* start with minimal: identifiers and quoted identifiers
* optionally parse typed params: `x as number`

### 5.2 Implementation details

Add:

* `parse_function_literal(tokens) -> Result<Option<MExpr>, MParseError>`

Algorithm:

1. Find top-level `=>`:

   * scan with delimiter depth (parens/brackets/braces)
   * detect `Symbol('=')` followed by `Symbol('>')` at depth 0
2. Confirm the head starts with a param list `(...)`:

   * find matching `)` for the first `(` at index 0
3. Parse params by splitting inner content on commas (use `split_top_level`)
4. Optionally parse return type if tokens between `)` and `=>` match `as <type>`
5. Parse body as `parse_expression(&tokens[arrow_idx+2..])`

### 5.3 Code skeleton

Add new code:

```rust
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

    // x
    if tokens.len() == 1 {
        if let Some(name) = token_as_name(&tokens[0]) {
            return Some(MParam { name, ty: None });
        }
        return None;
    }

    // x as number
    for i in 1..tokens.len() {
        if matches!(&tokens[i], MToken::Identifier(v) if v.eq_ignore_ascii_case("as")) {
            let name = token_as_name(&tokens[0])?;
            let ty = parse_type_ref(&tokens[i + 1..])?;
            return Some(MParam {
                name,
                ty: Some(ty),
            });
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

    // Find the matching ')' for the first '('
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

    // Parse params inside (...)
    let params_tokens = &tokens[1..close_paren];
    let param_slices = if params_tokens.is_empty() {
        Vec::new()
    } else {
        split_top_level(params_tokens, ',')
    };

    let mut params = Vec::new();
    for slice in param_slices {
        let slice = slice.iter().copied().filter(|_| true).collect::<Vec<_>>();
        let p = parse_param(&slice)?;
        params.push(p);
    }

    // Optional return type: (...) as <type> => <body>
    let mut return_type: Option<MTypeRef> = None;
    let between = &tokens[close_paren + 1..arrow_idx];
    if !between.is_empty() {
        if between.len() >= 2 && is_ident_token(&between[0], "as") {
            return_type = Some(parse_type_ref(&between[1..]).ok_or(MParseError::Empty)?);
        } else {
            return Ok(None);
        }
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
```

(That `parse_param_slices` “collect” line is intentionally conservative; you’ll likely just want a trim helper instead of allocating. The plan intent is: keep the parser strict enough to avoid false positives; otherwise return `Ok(None)` and let the caller fall back to `Opaque`.)

---

## Step 6 — `try <expr> otherwise <expr>` (optional but strongly recommended)

Branch 3 scope says “if you go that far.” 
In real queries, `try … otherwise …` is common; supporting it is a big semantic win.

### 6.1 Parsing approach

Since `try` is currently lexed as an identifier (lexer only special-cases `let/in/if/then/else/each`). 
We parse it as a special prefix form when:

* first token is Identifier("try") case-insensitive,
* and we can find an Identifier("otherwise") at top-level depth 0.

### 6.2 Add parser function

Add new code:

```rust
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
```

---

## Step 7 — The Tier‑2 “proper precedence parser” layer

This is the core of Branch 3. 

### 7.1 Operator table (precedence + associativity)

We need at least:

* unary: `not`, `+`, `-`
* multiplicative: `*`, `/`
* additive: `+`, `-`
* concatenation: `&`
* comparisons: `= <> < <= > >=`
* boolean: `and`, `or` 

A reasonable precedence ladder (lowest to highest) to start:

1. `or`
2. `and`
3. comparisons (`=`, `<>`, `<`, `<=`, `>`, `>=`)
4. concatenation `&`
5. additive `+ -`
6. multiplicative `* /`
7. unary (`not`, unary `+`, unary `-`)

(If M’s official precedence differs slightly, the structure makes it easy to reorder constants without rewriting parsing logic.)

### 7.2 How to implement without rewriting the whole parser

Given the existing design, the least risky approach is:

* parse “prefix constructs that consume the whole slice” first (`let`, parens strip, `if`, `each`, `try`, function literal), then…
* run an **infix split** algorithm that:

  * finds the best top-level operator (depth-aware),
  * splits into left/right slices,
  * recursively parses each side with `parse_expression`,
  * produces `BinaryOp` / `TypeAscription`,
  * if no infix op exists, tries unary prefix,
  * then falls back to existing primary forms (record/list/access/call/primitive/ident),
  * else opaque.

This is “precedence climbing” by recursive decomposition (choose lowest-precedence split point first), and it integrates cleanly with your current “best-effort decision tree” shape.

### 7.3 Critical nuance: don’t split on operators inside trailing `if/let/each/try/lambda`

Because constructs like `if … then … else …` are *not* delimited by parentheses, a naive top-level operator scan will incorrectly see operators inside the else-branch when the if-expression appears as the right operand (e.g., `a + if ... else y + z`). You already have special-casing logic for nesting in `parse_if_then_else`, but the operator split step must avoid “looking into” the tail expression.

The rule that makes this safe in your current architecture:

* When scanning for a split operator at depth 0, if you encounter the start of a “tail-consuming expression” (`if`, `let`, `each`, `try`, or a function literal head), **stop scanning beyond that point**, because everything after is part of the right operand expression slice.

This mirrors why `split_top_level` tracks `let_depth` to avoid splitting arguments inside nested `let`.

### 7.4 Implementation: best split operator finder

Add a small internal enum to represent a “split point”:

```rust
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
```

Add precedence constants (lower number = lower precedence = split first):

```rust
const PREC_OR: u8 = 10;
const PREC_AND: u8 = 20;
const PREC_CMP: u8 = 30;
const PREC_CONCAT: u8 = 40;
const PREC_ADD: u8 = 50;
const PREC_MUL: u8 = 60;
```

Then add:

```rust
fn is_tail_keyword(tok: &MToken) -> bool {
    matches!(tok, MToken::KeywordIf | MToken::KeywordLet | MToken::KeywordEach)
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

    let Some(close) = close else { return false; };
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

fn consider_best(best: &mut Option<SplitPoint>, cand: SplitPoint) {
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
                // Multi-token ops first
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
                        );
                        i += 2;
                        continue;
                    }
                }

                // Single-token symbol ops
                match &tokens[i] {
                    MToken::Symbol('+') => consider_best(
                        &mut best,
                        SplitPoint {
                            idx: i,
                            len: 1,
                            prec: PREC_ADD,
                            kind: InfixSplit::Binary(MBinaryOp::Add),
                        },
                    ),
                    MToken::Symbol('-') => consider_best(
                        &mut best,
                        SplitPoint {
                            idx: i,
                            len: 1,
                            prec: PREC_ADD,
                            kind: InfixSplit::Binary(MBinaryOp::Sub),
                        },
                    ),
                    MToken::Symbol('*') => consider_best(
                        &mut best,
                        SplitPoint {
                            idx: i,
                            len: 1,
                            prec: PREC_MUL,
                            kind: InfixSplit::Binary(MBinaryOp::Mul),
                        },
                    ),
                    MToken::Symbol('/') => consider_best(
                        &mut best,
                        SplitPoint {
                            idx: i,
                            len: 1,
                            prec: PREC_MUL,
                            kind: InfixSplit::Binary(MBinaryOp::Div),
                        },
                    ),
                    MToken::Symbol('&') => consider_best(
                        &mut best,
                        SplitPoint {
                            idx: i,
                            len: 1,
                            prec: PREC_CONCAT,
                            kind: InfixSplit::Binary(MBinaryOp::Concat),
                        },
                    ),
                    MToken::Symbol('=') => consider_best(
                        &mut best,
                        SplitPoint {
                            idx: i,
                            len: 1,
                            prec: PREC_CMP,
                            kind: InfixSplit::Binary(MBinaryOp::Eq),
                        },
                    ),
                    MToken::Symbol('<') => consider_best(
                        &mut best,
                        SplitPoint {
                            idx: i,
                            len: 1,
                            prec: PREC_CMP,
                            kind: InfixSplit::Binary(MBinaryOp::Lt),
                        },
                    ),
                    MToken::Symbol('>') => consider_best(
                        &mut best,
                        SplitPoint {
                            idx: i,
                            len: 1,
                            prec: PREC_CMP,
                            kind: InfixSplit::Binary(MBinaryOp::Gt),
                        },
                    ),
                    _ => {}
                }

                // Keyword-like ops (currently lexed as identifiers)
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
```

### 7.5 Parse unary + infix in a single function

Add:

```rust
fn parse_tier2_ops(tokens: &[MToken]) -> Result<Option<MExpr>, MParseError> {
    if let Some(split) = find_best_infix_split(tokens) {
        let left_tokens = &tokens[..split.idx];
        let right_tokens = &tokens[split.idx + split.len..];

        if left_tokens.is_empty() || right_tokens.is_empty() {
            return Ok(None);
        }

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
                    Some(t) => t,
                    None => return Ok(None),
                };
                return Ok(Some(MExpr::TypeAscription {
                    expr: Box::new(left),
                    ty,
                }));
            }
        }
    }

    // Unary operators (only if no infix split applies)
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
            // Preserve old behavior for -<number> as a primitive if possible
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
```

This approach:

* preserves existing negative-number literal parsing (`-1`) which is already supported by `parse_primitive`. 
* creates stable structured nodes for the tier‑2 operator cases required by the plan. 

---

## Step 8 — Wire everything into `parse_expression`

Today, `parse_expression` does:

* `parse_let`
* parens strip
* `parse_if_then_else`
* `parse_each_expr`
* record/list/access/call/primitive/ident
* else opaque 

We will:

1. keep the existing prefix constructs,
2. add function literal + try/otherwise,
3. add the tier‑2 precedence layer **before** record/list/access/call/primitive/ident,
4. then fall back as before.

#### Replace: current `parse_expression`

From `core/src/m_ast.rs` (excerpt): 

```rust
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
```

With:

```rust
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
```

---

## Step 9 — Update public docs (`parse_m_expression` comment)

The `parse_m_expression` doc comment currently lists supported constructs (Tier‑1 and earlier), and explicitly says other M constructs may be treated as generic tokens. 
Branch 3 adds new supported constructs, so update that comment to include:

* function literals,
* unary/binary ops,
* type ascription,
* try/otherwise.

This is “paper cut” but important for maintainability.

---

## Step 10 — Tests

### 10.1 Update coverage audit (Tier‑2 should no longer be opaque)

The current coverage audit test explicitly expects tier‑2 cases to remain opaque.

#### Replace: `coverage_audit_tier2_cases_remain_opaque`

From `core/tests/m8_m_parser_coverage_audit_tests.rs`: 

```rust
#[test]
fn coverage_audit_tier2_cases_remain_opaque() {
    let cases = ["(x) => x", "1 + 2", "not true", "x as number"];
    for expr in cases {
        assert_opaque(expr);
    }
}
```

With:

```rust
#[test]
fn coverage_audit_tier2_cases_are_structured() {
    assert_kind(
        "(x) => x",
        MAstKind::FunctionLiteral { param_count: 1 },
    );
    assert_kind("1 + 2", MAstKind::BinaryOp);
    assert_kind("not true", MAstKind::UnaryOp);
    assert_kind("x as number", MAstKind::TypeAscription);

    // Only include if TryOtherwise is implemented in this branch:
    // assert_kind("try 1 otherwise 0", MAstKind::TryOtherwise);
}
```

### 10.2 Add focused Tier‑2 parser tests

Create a new file: `core/tests/m10_m_parser_tier2_tests.rs` (name is flexible; the idea is “tier2”).

Pattern matches Tier‑1 tests style: parse, canonicalize, check `root_kind_for_testing()` or semantic equivalence.

Add tests:

1. **Function literal parsing**

* `(x) => x` is `FunctionLiteral { param_count: 1 }`
* `(x, y) => x + y` is `FunctionLiteral { param_count: 2 }`

2. **Unary ops**

* `not true` -> `UnaryOp`
* `-1` remains `Primitive` (or becomes UnaryOp if you prefer; but pick one and test it)
* `-(1)` -> `UnaryOp`

3. **Binary ops + precedence**

* Parse shapes via semantic equivalence instead of exposing op kinds:

  * `1 + 2 * 3` should be semantically equal to `1 + (2 * 3)`
  * `1 * 2 + 3` should be semantically equal to `(1 * 2) + 3`
  * `a or b and c` equals `a or (b and c)`

4. **Formatting-only semantic equality**

* `(x)=>x` vs `( x ) => x`
* `x as Number` vs `x as number` (type case normalization)
* `1+(2*3)` vs `1 + 2 * 3`

5. **Try/otherwise**

* `try 1 otherwise 0` parses and is stable under formatting changes:

  * `try (1) otherwise (0)` semantically equal

### 10.3 Example precedence test (recommended style)

Use semantic equality (since `ast_semantically_equal` is simply `a == b` after canonicalization).

Add:

```rust
use excel_diff::{ast_semantically_equal, canonicalize_m_ast, parse_m_expression};

fn canon(expr: &str) -> excel_diff::MModuleAst {
    let mut ast = parse_m_expression(expr).expect("parse should succeed");
    canonicalize_m_ast(&mut ast);
    ast
}

#[test]
fn precedence_mul_binds_tighter_than_add() {
    let a = canon("1 + 2 * 3");
    let b = canon("1 + (2 * 3)");
    assert!(ast_semantically_equal(&a, &b));
}

#[test]
fn precedence_and_binds_tighter_than_or() {
    let a = canon("a or b and c");
    let b = canon("a or (b and c)");
    assert!(ast_semantically_equal(&a, &b));
}
```

---

## Definition of done for Branch 3 (concrete checklist)

Aligned to the plan’s intent and your codebase’s existing invariants:

1. **Tier‑2 cases are no longer opaque**

* `(x) => x` => `MAstKind::FunctionLiteral`
* `1 + 2` => `MAstKind::BinaryOp`
* `not true` => `MAstKind::UnaryOp`
* `x as number` => `MAstKind::TypeAscription`

2. **Operator precedence works**

* `1 + 2 * 3` semantically equals `1 + (2 * 3)` 

3. **Formatting-only invariants still hold**

* new canonicalization is stable/idempotent and masks whitespace/parentheses-only differences (same way you already test for formatting-only fixtures and canonicalization idempotency).

4. **No regressions to Tier‑1**

* existing Tier‑1 tests (ident/access/if/each) still pass.

