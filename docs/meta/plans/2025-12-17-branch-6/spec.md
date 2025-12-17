Below is an implementation plan that takes your current codebase from where it is today (semantic M diff works, but the M AST/parser only structurally understands `let … in …` and otherwise treats expressions as an opaque token sequence) through **completion of Branch 6: “M Parser Expansion”** as specified in `next_sprint_plan.md`.  

---

## What Branch 6 requires (definition of done)

Branch 6 asks for three things: 

1. **Audit current parser coverage**, document what forms are parsed vs treated as opaque, and prioritize by frequency.
2. **Extend the M parser** beyond `let` to produce AST nodes for:

   * record literals: `[Field1 = 1, Field2 = 2]`
   * list literals: `{1,2,3}`
   * direct function calls: `Table.FromRows(.)`
   * direct primitives: `"hello"`, `42`
   * keep an **opaque fallback** for everything else
3. **Add semantic equivalence tests**

   * specifically include **record-field order insensitivity**: `[B=2, A=1]` equals `[A=1, B=2]`
   * ensure canonicalization is meaningful for non-let expressions (records sorted, lists preserved)
   * no regressions in existing M diff tests

---

## Current baseline (so the plan is grounded in your code)

Right now, `core/src/m_ast.rs` does the following: 

* Tokenizes M, stripping whitespace and comments.
* Parses **only** `let … in …` into a structural AST node.
* Everything else becomes `Sequence(Vec<MToken>)` (effectively opaque).
* Canonicalization for non-let expressions is effectively a no-op (it just keeps the token sequence). 

Your semantic M diff hashes a canonicalized AST when enabled, and uses those hashes to mark changes as semantic vs formatting-only. 

You also already have a fixture generator and a manifest-driven fixture set, including milestone 7 “M AST canonicalization” scenarios (`m_formatting_only_a/b/...`).  

Branch 6 is therefore primarily a focused upgrade to **`m_ast.rs` + fixtures + tests**, without touching the larger diff engine.

---

## Step-by-step implementation plan

### Step 1: Add the Branch 6 “coverage audit” doc (6.1)

**Files**

* Add: `docs/m_parser_coverage.md` (or `core/docs/m_parser_coverage.md` if you keep docs near the crate)

**What to include**

* A table or bullet list of:

  * **Parsed to AST**:

    * `let … in …`
    * record literals `[...]` with `name = expr` fields
    * list literals `{...}`
    * qualified function calls like `Table.FromRows(...)`
    * primitives: string, number (+ optionally `true/false/null`)
  * **Opaque fallback**:

    * everything else (operators, indexing like `{...}[Content]`, `if/then/else`, `each`, etc.)

**Prioritization by frequency**

* In Power Query workbooks, most query roots are `let … in …`, and within them a lot of structure is table/list/record construction, with frequent use of qualified function calls (e.g., `Excel.CurrentWorkbook`, `Table.*`, `Value.*`). Branch 6 targets the most common “non-let roots” and the most canonicalizable structure (record field order). 

**Optional but useful instrumentation**

* Add a small dev-only example (`core/examples/m_ast_coverage.rs`) that:

  * scans a directory of `.xlsx`
  * extracts query expressions (you already have the DataMashup parsing pipeline)
  * parses each expression to `MModuleAst`
  * counts `root_kind_for_testing()` buckets
* This gives you “real” frequency data without needing a full M grammar.

**New file content (doc)**

```md
# M parser coverage (Branch 6)

## Parsed structural forms
- let ... in ...
- record literal: [Field = Expr, ...]
- list literal: {Expr, ...}
- function call: Name(...) where Name is Identifier(.Identifier)*
- primitive: string, number, true/false/null

## Opaque fallback
Everything else is represented as an opaque token sequence.
Examples:
- field access: [Content]
- indexing: Source{0}[Column]
- operators: 1 + 2
- if/then/else
- each/lambda
- type annotations
```

---

### Step 2: Extend the AST types (6.2)

**Files**

* Modify: `core/src/m_ast.rs` 

**Goal**

* Replace “Sequence-only non-let” with first-class nodes:

  * `Record`
  * `List`
  * `FunctionCall`
  * `Primitive`
  * `Opaque` (renamed from Sequence, conceptually)

#### Code change: AST definitions

**Replace this existing block** (from `core/src/m_ast.rs`): 

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MAstKind {
    Let { binding_count: usize },
    Sequence { token_count: usize },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MExpr {
    Let {
        bindings: Vec<LetBinding>,
        body: Box<MExpr>,
    },
    Sequence(Vec<MToken>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct LetBinding {
    name: String,
    value: Box<MExpr>,
}
```

**With this**:

```rust
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
```

#### Code change: `root_kind_for_testing`

**Replace this** (current): 

```rust
pub fn root_kind_for_testing(&self) -> MAstKind {
    match &self.root {
        MExpr::Let { bindings, .. } => MAstKind::Let {
            binding_count: bindings.len(),
        },
        MExpr::Sequence(tokens) => MAstKind::Sequence {
            token_count: tokens.len(),
        },
    }
}
```

**With this**:

```rust
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
```

---

### Step 3: Implement parsing for record/list/call/primitive with robust comma splitting (6.2)

This is the most important “gotcha” in Branch 6:

* commas are not only used in lists/records/args
* they also appear inside nested `let` binding lists **without parentheses**
* so your “split by commas at depth 0” logic must track both:

  * delimiter nesting depth (`()[]{}`)
  * nested `let` depth (`let` increments, `in` decrements)
    You already had to do this in `parse_let` (using `let_depth_in_value`). 
    You need the same idea for record/list/args splitting.

#### Code change: parsing entry point + helper parsers

**Replace the existing `parse_expression` + `parse_let` block** with the following unified block. (This avoids “insert new functions here” instructions and keeps the change copy/pasteable.)

**Code to be replaced** (the current `fn parse_expression` + `fn parse_let` block): 

```rust
fn parse_expression(tokens: &[MToken]) -> Result<MExpr, MParseError> {
    if let Some(let_ast) = parse_let(tokens)? {
        return Ok(let_ast);
    }
    Ok(MExpr::Sequence(tokens.to_vec()))
}

fn parse_let(tokens: &[MToken]) -> Result<Option<MExpr>, MParseError> {
    if tokens.is_empty() {
        return Err(MParseError::EmptyExpression);
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
        return Err(MParseError::MissingInKeyword);
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
```

**New code to replace it**:

```rust
fn parse_expression(tokens: &[MToken]) -> Result<MExpr, MParseError> {
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
        return Err(MParseError::EmptyExpression);
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
        return Err(MParseError::MissingInKeyword);
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
```

**Why these heuristics are safe**

* Record parsing only triggers for `[ ... ]` where every comma-separated part matches `Name = Value`.

  * This avoids misparsing `[Content]` (field access) as a record literal.
* `split_top_level` uses both delimiter nesting and nested-let tracking, so commas inside `let` bindings won’t break record/list/args splitting.

---

### Step 4: Implement canonicalization for new nodes (6.3)

**Files**

* Modify: `core/src/m_ast.rs` canonicalization functions 

**Goal**

* Records: canonicalize values and **sort fields by name**
* Lists: canonicalize each item, preserve order
* Function call: canonicalize args
* Primitive: unchanged
* Opaque: unchanged

#### Code change: canonicalization

**Replace this existing block**: 

```rust
fn canonicalize_expr(expr: &mut MExpr) {
    match expr {
        MExpr::Let { bindings, body } => {
            for binding in bindings.iter_mut() {
                canonicalize_expr(&mut binding.value);
            }
            canonicalize_expr(body);
        }
        MExpr::Sequence(tokens) => canonicalize_tokens(tokens),
    }
}

fn canonicalize_tokens(tokens: &mut Vec<MToken>) {
    let _ = tokens;
}
```

**With this**:

```rust
fn canonicalize_expr(expr: &mut MExpr) {
    match expr {
        MExpr::Let { bindings, body } => {
            for binding in bindings.iter_mut() {
                canonicalize_expr(&mut binding.value);
            }
            canonicalize_expr(body);
        }
        MExpr::Record { fields } => {
            for f in fields.iter_mut() {
                canonicalize_expr(&mut f.value);
            }
            fields.sort_by(|a, b| a.name.cmp(&b.name));
        }
        MExpr::List { items } => {
            for item in items.iter_mut() {
                canonicalize_expr(item);
            }
        }
        MExpr::FunctionCall { args, .. } => {
            for arg in args.iter_mut() {
                canonicalize_expr(arg);
            }
        }
        MExpr::Primitive(_) => {}
        MExpr::Opaque(tokens) => canonicalize_tokens(tokens),
    }
}

fn canonicalize_tokens(tokens: &mut Vec<MToken>) {
    let _ = tokens;
}
```

This directly implements the Branch 6 semantic-equivalence requirement for record field order. 

---

### Step 5: Add unit tests for new parse forms + semantic equivalence (6.3)

**Files**

* Add: `core/tests/m8_m_parser_expansion_tests.rs`

This gives fast feedback without needing fixture generation.

**New file (empty -> new):**

```rust
```

```rust
use excel_diff::{ast_semantically_equal, canonicalize_m_ast, parse_m_expression, MAstKind};

#[test]
fn record_literal_parses_as_record() {
    let ast = parse_m_expression("[Field1 = 1, Field2 = 2]").unwrap();
    assert_eq!(ast.root_kind_for_testing(), MAstKind::Record { field_count: 2 });
}

#[test]
fn list_literal_parses_as_list() {
    let ast = parse_m_expression("{1,2,3}").unwrap();
    assert_eq!(ast.root_kind_for_testing(), MAstKind::List { item_count: 3 });
}

#[test]
fn function_call_parses_as_call() {
    let ast = parse_m_expression("Table.FromRows(.)").unwrap();
    assert_eq!(
        ast.root_kind_for_testing(),
        MAstKind::FunctionCall { name: "Table.FromRows".to_string(), arg_count: 1 }
    );
}

#[test]
fn primitive_string_parses() {
    let ast = parse_m_expression(r#""hello""#).unwrap();
    assert_eq!(ast.root_kind_for_testing(), MAstKind::Primitive);
}

#[test]
fn primitive_number_parses() {
    let ast = parse_m_expression("42").unwrap();
    assert_eq!(ast.root_kind_for_testing(), MAstKind::Primitive);
}

#[test]
fn record_field_order_is_semantically_equivalent() {
    let mut a = parse_m_expression("[B=2, A=1]").unwrap();
    let mut b = parse_m_expression("[A=1, B=2]").unwrap();

    canonicalize_m_ast(&mut a);
    canonicalize_m_ast(&mut b);

    assert!(ast_semantically_equal(&a, &b));
}

#[test]
fn list_order_is_not_semantically_equivalent() {
    let mut a = parse_m_expression("{1,2}").unwrap();
    let mut b = parse_m_expression("{2,1}").unwrap();

    canonicalize_m_ast(&mut a);
    canonicalize_m_ast(&mut b);

    assert!(!ast_semantically_equal(&a, &b));
}
```

---

### Step 6: Add fixture scenarios for each new construct (6.1 + acceptance)

Branch 6 explicitly asks for fixtures covering each construct that was previously unsupported. 

You already generate mashup fixtures through `fixtures/manifest.yaml` and `MashupPermissionsMetadataGenerator`’s `mode` switch.  

#### 6.1 Add new generator modes in `fixtures/src/generators/mashup.py`

**Files**

* Modify: `fixtures/src/generators/mashup.py` 

**Where**

* Inside `MashupPermissionsMetadataGenerator._scenario_definition` near other `m_*` modes.

**Replace this block end** (the existing formatting-only mode portion) with an extended version that adds Branch 6 scenarios.

You currently have a helper that builds a single query named `FormatTest` with a chosen body for formatting-only scenarios. 

**Code to be replaced** (the `m_formatting_only_*` branch): 

```python
        if self.mode == "m_formatting_only_a":
            body = "\n".join(
                [
                    "let",
                    "    Source = 1",
                    "in",
                    "    Source",
                ]
            )
            return m_diff_scenario(
                [
                    {
                        "name": "FormatTest",
                        "body": body,
                        "load_to_sheet": True,
                        "load_to_model": False,
                    },
                ]
            )

        if self.mode == "m_formatting_only_b":
            body = "\n".join(
                [
                    "let",
                    "    Source = 1,",
                    "    Foo = 2",
                    "in",
                    "    Source",
                ]
            )
            return m_diff_scenario(
                [
                    {
                        "name": "FormatTest",
                        "body": body,
                        "load_to_sheet": True,
                        "load_to_model": False,
                    },
                ]
            )

        if self.mode == "m_formatting_only_b_variant":
            body = "\n".join(
                [
                    "let",
                    "    Source = 1,",
                    "    Foo = 3",
                    "in",
                    "    Source",
                ]
            )
            return m_diff_scenario(
                [
                    {
                        "name": "FormatTest",
                        "body": body,
                        "load_to_sheet": True,
                        "load_to_model": False,
                    },
                ]
            )
```

**New code to replace it** (same existing modes + new Branch 6 modes):

```python
        if self.mode == "m_formatting_only_a":
            body = "\n".join(
                [
                    "let",
                    "    Source = 1",
                    "in",
                    "    Source",
                ]
            )
            return m_diff_scenario(
                [
                    {
                        "name": "FormatTest",
                        "body": body,
                        "load_to_sheet": True,
                        "load_to_model": False,
                    },
                ]
            )

        if self.mode == "m_formatting_only_b":
            body = "\n".join(
                [
                    "let",
                    "    Source = 1,",
                    "    Foo = 2",
                    "in",
                    "    Source",
                ]
            )
            return m_diff_scenario(
                [
                    {
                        "name": "FormatTest",
                        "body": body,
                        "load_to_sheet": True,
                        "load_to_model": False,
                    },
                ]
            )

        if self.mode == "m_formatting_only_b_variant":
            body = "\n".join(
                [
                    "let",
                    "    Source = 1,",
                    "    Foo = 3",
                    "in",
                    "    Source",
                ]
            )
            return m_diff_scenario(
                [
                    {
                        "name": "FormatTest",
                        "body": body,
                        "load_to_sheet": True,
                        "load_to_model": False,
                    },
                ]
            )

        if self.mode == "m_record_equiv_a":
            return m_diff_scenario(
                [
                    {
                        "name": "RecordRoot",
                        "body": "[B=2, A=1]",
                        "load_to_sheet": True,
                        "load_to_model": False,
                    },
                ]
            )

        if self.mode == "m_record_equiv_b":
            return m_diff_scenario(
                [
                    {
                        "name": "RecordRoot",
                        "body": "[A=1, B=2]",
                        "load_to_sheet": True,
                        "load_to_model": False,
                    },
                ]
            )

        if self.mode == "m_list_formatting_a":
            return m_diff_scenario(
                [
                    {
                        "name": "ListRoot",
                        "body": "{1,2,3}",
                        "load_to_sheet": True,
                        "load_to_model": False,
                    },
                ]
            )

        if self.mode == "m_list_formatting_b":
            return m_diff_scenario(
                [
                    {
                        "name": "ListRoot",
                        "body": "{ 1, /*c*/ 2, 3 }",
                        "load_to_sheet": True,
                        "load_to_model": False,
                    },
                ]
            )

        if self.mode == "m_call_formatting_a":
            return m_diff_scenario(
                [
                    {
                        "name": "CallRoot",
                        "body": "Table.FromRows({{1,2},{3,4}}, {\"A\",\"B\"})",
                        "load_to_sheet": True,
                        "load_to_model": False,
                    },
                ]
            )

        if self.mode == "m_call_formatting_b":
            body = "\n".join(
                [
                    "Table.FromRows(",
                    "    {{1,2},{3,4}},",
                    "    {\"A\", \"B\"}",
                    ")",
                ]
            )
            return m_diff_scenario(
                [
                    {
                        "name": "CallRoot",
                        "body": body,
                        "load_to_sheet": True,
                        "load_to_model": False,
                    },
                ]
            )

        if self.mode == "m_primitive_formatting_a":
            return m_diff_scenario(
                [
                    {
                        "name": "PrimRoot",
                        "body": "\"hello\"",
                        "load_to_sheet": True,
                        "load_to_model": False,
                    },
                ]
            )

        if self.mode == "m_primitive_formatting_b":
            return m_diff_scenario(
                [
                    {
                        "name": "PrimRoot",
                        "body": "   \"hello\"   ",
                        "load_to_sheet": True,
                        "load_to_model": False,
                    },
                ]
            )
```

#### 6.2 Add the new scenarios to `fixtures/manifest.yaml`

**Files**

* Modify: `fixtures/manifest.yaml` 

You currently end milestone 7 with `m_formatting_only_*`. 
Add a new “Milestone 8: M parser expansion” section right after.

**Code to be replaced** (the tail of milestone 7): 

```yaml
  - id: "m_formatting_only_b_variant"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_formatting_only_b_variant"
      base_file: "templates/base_query.xlsx"
    output: "m_formatting_only_b_variant.xlsx"
```

**New code to replace it** (same + appended scenarios):

```yaml
  - id: "m_formatting_only_b_variant"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_formatting_only_b_variant"
      base_file: "templates/base_query.xlsx"
    output: "m_formatting_only_b_variant.xlsx"

  # --- Milestone 8: M Parser Expansion ---
  - id: "m_record_equiv_a"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_record_equiv_a"
      base_file: "templates/base_query.xlsx"
    output: "m_record_equiv_a.xlsx"

  - id: "m_record_equiv_b"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_record_equiv_b"
      base_file: "templates/base_query.xlsx"
    output: "m_record_equiv_b.xlsx"

  - id: "m_list_formatting_a"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_list_formatting_a"
      base_file: "templates/base_query.xlsx"
    output: "m_list_formatting_a.xlsx"

  - id: "m_list_formatting_b"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_list_formatting_b"
      base_file: "templates/base_query.xlsx"
    output: "m_list_formatting_b.xlsx"

  - id: "m_call_formatting_a"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_call_formatting_a"
      base_file: "templates/base_query.xlsx"
    output: "m_call_formatting_a.xlsx"

  - id: "m_call_formatting_b"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_call_formatting_b"
      base_file: "templates/base_query.xlsx"
    output: "m_call_formatting_b.xlsx"

  - id: "m_primitive_formatting_a"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_primitive_formatting_a"
      base_file: "templates/base_query.xlsx"
    output: "m_primitive_formatting_a.xlsx"

  - id: "m_primitive_formatting_b"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_primitive_formatting_b"
      base_file: "templates/base_query.xlsx"
    output: "m_primitive_formatting_b.xlsx"
```

#### 6.3 Regenerate fixtures

Your fixture generator uses `fixtures/manifest.yaml` by default and writes to `fixtures/generated`. 

Recommended commands (from repo root):

* Install fixture tooling once (if you don’t already):

  * `pip install -e fixtures`
* Generate:

  * `generate-fixtures --force`

---

### Step 7: Add end-to-end semantic diff tests for the new forms (acceptance)

You already have semantic M diff tests that load packages from fixtures and check `QueryDefinitionChanged` is marked formatting-only when canonical hashes match. 

Add similar tests for the new fixture pairs.

**Files**

* Add: `core/tests/m8_semantic_m_diff_nonlet_tests.rs`

**New file (empty -> new):**

```rust
```

```rust
use excel_diff::{diff_workbook_packages, DiffConfig, DiffOp, QueryChangeKind, WorkbookPackage};
use excel_diff::test_fixture_path;

fn load_pkg(name: &str) -> WorkbookPackage {
    let p = test_fixture_path(name);
    WorkbookPackage::open(&p).expect("open pkg")
}

fn m_ops<'a>(ops: &'a [DiffOp]) -> Vec<&'a DiffOp> {
    ops.iter()
        .filter(|op| matches!(op, DiffOp::QueryAdded { .. }
            | DiffOp::QueryRemoved { .. }
            | DiffOp::QueryRenamed { .. }
            | DiffOp::QueryDefinitionChanged { .. }))
        .collect()
}

#[test]
fn record_reorder_is_masked_by_semantic_canonicalization() {
    let a = load_pkg("m_record_equiv_a.xlsx");
    let b = load_pkg("m_record_equiv_b.xlsx");

    let cfg = DiffConfig { enable_m_semantic_diff: true, ..Default::default() };
    let diff = diff_workbook_packages(&a, &b, &cfg).expect("diff");

    let ops = m_ops(&diff.ops);
    assert_eq!(ops.len(), 1);

    match ops[0] {
        DiffOp::QueryDefinitionChanged { change_kind, old_hash, new_hash, .. } => {
            assert_eq!(*change_kind, QueryChangeKind::FormattingOnly);
            assert_eq!(old_hash, new_hash);
        }
        _ => panic!("expected QueryDefinitionChanged"),
    }
}

#[test]
fn list_formatting_only_is_masked() {
    let a = load_pkg("m_list_formatting_a.xlsx");
    let b = load_pkg("m_list_formatting_b.xlsx");

    let cfg = DiffConfig { enable_m_semantic_diff: true, ..Default::default() };
    let diff = diff_workbook_packages(&a, &b, &cfg).expect("diff");

    let ops = m_ops(&diff.ops);
    assert_eq!(ops.len(), 1);

    match ops[0] {
        DiffOp::QueryDefinitionChanged { change_kind, old_hash, new_hash, .. } => {
            assert_eq!(*change_kind, QueryChangeKind::FormattingOnly);
            assert_eq!(old_hash, new_hash);
        }
        _ => panic!("expected QueryDefinitionChanged"),
    }
}

#[test]
fn call_formatting_only_is_masked() {
    let a = load_pkg("m_call_formatting_a.xlsx");
    let b = load_pkg("m_call_formatting_b.xlsx");

    let cfg = DiffConfig { enable_m_semantic_diff: true, ..Default::default() };
    let diff = diff_workbook_packages(&a, &b, &cfg).expect("diff");

    let ops = m_ops(&diff.ops);
    assert_eq!(ops.len(), 1);

    match ops[0] {
        DiffOp::QueryDefinitionChanged { change_kind, old_hash, new_hash, .. } => {
            assert_eq!(*change_kind, QueryChangeKind::FormattingOnly);
            assert_eq!(old_hash, new_hash);
        }
        _ => panic!("expected QueryDefinitionChanged"),
    }
}

#[test]
fn primitive_formatting_only_is_masked() {
    let a = load_pkg("m_primitive_formatting_a.xlsx");
    let b = load_pkg("m_primitive_formatting_b.xlsx");

    let cfg = DiffConfig { enable_m_semantic_diff: true, ..Default::default() };
    let diff = diff_workbook_packages(&a, &b, &cfg).expect("diff");

    let ops = m_ops(&diff.ops);
    assert_eq!(ops.len(), 1);

    match ops[0] {
        DiffOp::QueryDefinitionChanged { change_kind, old_hash, new_hash, .. } => {
            assert_eq!(*change_kind, QueryChangeKind::FormattingOnly);
            assert_eq!(old_hash, new_hash);
        }
        _ => panic!("expected QueryDefinitionChanged"),
    }
}
```

This set directly covers the Branch 6 acceptance that semantic diff “correctly identifies formatting-only changes for all forms” and that record field reordering is treated as semantically equivalent. 

---

## Final verification checklist (to ensure Branch 6 is actually complete)

1. **Run core tests**

   * `cargo test -p core`
2. **Regenerate fixtures and commit updated `fixtures/generated/*.xlsx`**

   * `generate-fixtures --force` 
3. **Run semantic M diff tests**

   * Ensure existing milestone 7 tests still pass (no regressions) 
   * Ensure the new milestone 8 tests pass
4. **Review the coverage doc**

   * Make sure it clearly distinguishes what’s parsed vs opaque and why (and includes prioritization)

---

## Notes on scope (what Branch 6 does not attempt)

Branch 6 does **not** require a full M grammar (operators, indexing chains, `if/then/else`, `each`, etc.). Those remain in the opaque fallback, which is explicitly allowed by the spec. 

The key value here is that once records/lists/calls/primitives exist structurally, you can start adding “semantic normalization wins” (like record field sorting) in a controlled way, without risking the rest of the language.

---

If you want, I can also propose the next incremental “Branch 7-ish” upgrades after this (e.g., parsing `{ ... }[Field]` postfix access patterns safely, or canonicalizing `#date(...)` as a primitive), but the plan above is sufficient to hit **Branch 6 completion exactly as written**.
