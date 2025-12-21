## Branch 2 — `2025-12-23-m-parser-tier1-ident-access`

### What this branch is meant to unlock

Right now the M parser intentionally recognizes only a small “spine” (`let … in …`, record/list literals, qualified function calls, primitives) and then preserves everything else as `Opaque(tokens)` . Branch 2’s explicit goal is to turn the most common *currently opaque* constructs—identifier references, access chains, `if`, `each`—into structured nodes so canonicalization + semantic equality can operate on meaning instead of token soup. 

That matters directly to the diff pipeline: semantic diff classification uses `parse_m_expression` → `canonicalize_m_ast` → hash of the AST to decide `FormattingOnly` vs `Semantic`. 

The coverage audit test explicitly lists these constructs as currently opaque and expects `MAstKind::Opaque`.  Branch 2’s “definition of done” says those Tier‑1 cases should stop being opaque and assert the correct `MAstKind`. 

---

## Design constraints to preserve (so this doesn’t destabilize the project)

1. **Best-effort behavior stays**
   No new hard parse failures for “weird but valid” M; if a Tier‑1 parser doesn’t match cleanly, fall back to `Opaque(tokens)` (consistent with current design). 

2. **Keep AST opaque to consumers**
   The AST shape is intentionally not exposed; tests get a minimal `MAstKind` view via `root_kind_for_testing()`. 
   We’ll extend `MAstKind` enough to assert Tier‑1 structure, without making the whole tree public.

3. **Canonicalization remains idempotent and stable**
   There’s an explicit test that canonicalization is idempotent. 
   Any new node types must be canonicalized deterministically (mostly: recurse, don’t reorder unless intended).

---

## Implementation plan (step-by-step)

### Step 1 — Extend the lexer with minimal keyword tokens (important for correctness)

#### Why this is worth doing

Today, `if/then/else/each` arrive as `Identifier("if")`, etc. That is *ambiguous* with quoted identifiers because the lexer intentionally strips `#"..."` into a plain `Identifier(inner)` token (e.g., `#"Foo"` becomes `Identifier("Foo")`). 

A real-world ambiguous case you will hit:

* `if #"then" then 1 else 0`
  If you keep `then` as just an identifier token, the parser can easily confuse the quoted identifier `#"then"` with the `then` keyword.

Adding keyword tokens resolves this cleanly because:

* unquoted `then` becomes `KeywordThen`
* quoted `#"then"` stays `Identifier("then")` (because quoted identifiers are handled in a distinct codepath) 

#### What to change

Add four token variants:

* `KeywordIf`
* `KeywordThen`
* `KeywordElse`
* `KeywordEach`

Update:

* `enum MToken`
* `pub enum MTokenDebug`
* `impl From<&MToken> for MTokenDebug`
* identifier classification inside `tokenize(...)` 

#### Code replacement snippet (tokens + debug mirror)

Code to replace (current shape): 

```rust
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
```

New code to replace it with:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MToken {
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
```

#### Code replacement snippet (identifier classification inside `tokenize`)

In `tokenize`, there is a section that maps `let/in` to keyword tokens. 

Code to replace (current mapping):

```rust
let token = if ident.eq_ignore_ascii_case("let") {
    MToken::KeywordLet
} else if ident.eq_ignore_ascii_case("in") {
    MToken::KeywordIn
} else {
    MToken::Identifier(ident)
};
tokens.push(token);
continue;
```

New code:

```rust
let token = if ident.eq_ignore_ascii_case("let") {
    MToken::KeywordLet
} else if ident.eq_ignore_ascii_case("in") {
    MToken::KeywordIn
} else if ident.eq_ignore_ascii_case("if") {
    MToken::KeywordIf
} else if ident.eq_ignore_ascii_case("then") {
    MToken::KeywordThen
} else if ident.eq_ignore_ascii_case("else") {
    MToken::KeywordElse
} else if ident.eq_ignore_ascii_case("each") {
    MToken::KeywordEach
} else {
    MToken::Identifier(ident)
};
tokens.push(token);
continue;
```

#### Follow-on compatibility tweaks (recommended)

Because keyword tokens are now distinct, update name-parsing sites to accept them where “identifier-like” is valid and unambiguous:

* `parse_record_literal`: field names currently accept `Identifier` or `StringLiteral` . Allow keyword tokens too, so `[if = 1]` doesn’t degrade into opaque.
* (Optional) access keys: if inside brackets you have a single keyword token (e.g., `Source[if]`), you’ll want it to parse as an identifier reference inside the bracket rather than failing.

You can do this cleanly by adding a tiny helper:

```rust
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
```

---

### Step 2 — Add Tier‑1 AST variants (internal) + a minimal public MAstKind view

Branch plan explicitly calls out adding these node types: ident refs, if, each, access with chaining. 

#### New internal AST shape (MExpr)

Add:

* `Ident { name }`
* `If { cond, then_branch, else_branch }`
* `Each { body }`
* `Access { base, kind, key }`

  * `kind` is `Field` (`[]`) vs `Item` (`{}`) 

#### Public `MAstKind` changes for tests

Current `MAstKind` only reflects the current spine plus `Opaque`. 
Update it to include:

* `Ident { name }`
* `If`
* `Each`
* `Access { kind, chain_len }`

Where:

* `chain_len` is the number of access hops, so `Source{0}[Content]` can assert `chain_len == 2`.
* `kind` is the outermost hop kind (Field vs Item).

This keeps the public surface minimal while allowing the exact assertions Branch 2 wants (coverage audit + focused tests). 

#### Code replacement snippet (MAstKind + MExpr + root_kind_for_testing)

Code to replace (current shape): 

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

impl MModuleAst {
    pub fn root_kind_for_testing(&self) -> MAstKind {
        match &self.root {
            MExpr::Let { bindings, . } => MAstKind::Let {
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
```

New code to replace it with:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MAstAccessKind {
    Field,
    Item,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum AccessKind {
    Field,
    Item,
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

impl MModuleAst {
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
}
```

---

### Step 3 — Canonicalization: recurse through Tier‑1 nodes

Current canonicalization:

* recurses through let/record/list/call
* sorts record fields
* canonicalizes boolean/null token casing only inside `Opaque(tokens)` 

Add recursion for new nodes:

* `Ident`: no-op
* `If`: recurse into `cond`, `then_branch`, `else_branch`
* `Each`: recurse into `body`
* `Access`: recurse into `base` and `key`

This preserves existing invariants while enabling canonical semantics for these new constructs.

#### Code replacement snippet (canonicalize_expr)

Code to replace (existing match): 

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
        MExpr::FunctionCall { args, . } => {
            for arg in args {
                canonicalize_expr(arg);
            }
        }
        MExpr::Primitive(_) => {}
        MExpr::Opaque(tokens) => canonicalize_tokens(tokens),
    }
}
```

New code:

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
```

**Note on boolean/null canonicalization:**
The sprint plan suggests moving boolean/null normalization from “opaque-token canonicalization” into real AST canonicalization where possible. 
You get this “for free” for conditionals because `TRUE` and `true` parse into the same `Primitive(Boolean(true))` today.  So you likely don’t need extra work beyond ensuring conditionals are parsed structurally.

---

### Step 4 — Parser: add Tier‑1 parse functions and integrate them into the decision tree

Branch 2 explicitly wants to keep the current best-effort “try parse X” decision tree and extend it. 
Today the order is:
`let → strip parens → record/list → call → primitive → opaque` 

We’ll extend it with:

* `if`
* `each`
* `access chain`
* `identifier ref`

#### Proposed decision tree order

1. `let`
2. strip wrapping parens
3. `if`
4. `each`
5. record literal
6. list literal
7. access chain
8. function call
9. primitive
10. identifier reference
11. opaque

This order keeps the current “spine” intact and ensures:

* primitives win over ident refs (`true`, `false`, `null`) 
* access chains can wrap any base expression (including call, record, list, let) without requiring “parse-one-and-return-consumed-len” machinery

---

## Step 4a — Identifier references

**Goal:** parse `Source` and `#"Previous Step"` as `MExpr::Ident { name }` instead of `Opaque`. 

Implementation:

* if `tokens.len() == 1` and token is `Identifier(s)` → Ident
* also accept keyword tokens as ident refs for robustness (`KeywordIf`, etc.)

Example helper:

```rust
fn parse_ident_ref(tokens: &[MToken]) -> Option<MExpr> {
    if tokens.len() != 1 {
        return None;
    }

    match &tokens[0] {
        MToken::Identifier(v) => Some(MExpr::Ident { name: v.clone() }),
        MToken::KeywordIf => Some(MExpr::Ident { name: "if".to_string() }),
        MToken::KeywordThen => Some(MExpr::Ident { name: "then".to_string() }),
        MToken::KeywordElse => Some(MExpr::Ident { name: "else".to_string() }),
        MToken::KeywordEach => Some(MExpr::Ident { name: "each".to_string() }),
        _ => None,
    }
}
```

---

## Step 4b — Each expressions

**Goal:** parse `each <expr>` as `MExpr::Each { body }` instead of opaque. 

Implementation:

* match first token `KeywordEach`
* require at least one token after it
* parse remainder with `parse_expression`

```rust
fn parse_each_expr(tokens: &[MToken]) -> Result<Option<MExpr>, MParseError> {
    if !matches!(tokens.first(), Some(MToken::KeywordEach)) {
        return Ok(None);
    }
    if tokens.len() < 2 {
        return Ok(None);
    }

    let body = parse_expression(&tokens[1..])?;
    Ok(Some(MExpr::Each { body: Box::new(body) }))
}
```

---

## Step 4c — If/then/else

**Goal:** parse `if <cond> then <a> else <b>` structurally. 

Implementation strategy:

* match first token `KeywordIf`
* scan tokens to find the **top-level** `KeywordThen` and `KeywordElse`

  * track delimiter depth over `()[]{}` so you ignore `then/else` inside parentheses, records, lists, etc.
  * track `let_depth` (like existing `split_top_level`) to ignore keywords inside nested `let` blocks 
  * track `if_depth` to avoid mistaking nested `if`’s then/else for the outer one

Return `Ok(None)` if the shape isn’t clean; let the caller fall back to `Opaque`.

```rust
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

    let Some(then_idx) = then_idx else { return Ok(None); };
    let Some(else_idx) = else_idx else { return Ok(None); };

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
```

---

## Step 4d — Access chains (`[]` and `{}`) with chaining support

**Goal:** parse:

* `Source[Field]` (field access)
* `Source{0}` (item access)
* `Source{0}[Content]` (chained) 

### The approach that fits your current parser

Since `parse_expression` consumes the whole slice and doesn’t return a “consumed length,” the simplest robust method is:

1. **Peel suffix segments from the end** while the expression ends with `]` or `}`:

   * if ends with `]`, find the matching `[` that begins that final segment
   * if ends with `}`, find matching `{`
   * store the inner tokens for that segment as the “key”
   * reduce the remaining token slice and repeat (this naturally supports chaining)

2. When you’re done peeling:

   * parse the remaining prefix as the base expression
   * apply peeled segments in reverse order to build nested `Access` nodes

This avoids confusion with record/list literals because a record literal alone would peel to an empty base, which you reject (falling back to record parsing). But a record literal followed by access (e.g., `[a=1][a]`) works naturally: you peel the final `[a]` segment and then parse `[a=1]` as the base.

```rust
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

        let (kind, open_ch, close_ch) = match tokens[end - 1] {
            MToken::Symbol(']') => (AccessKind::Field, '[', ']'),
            MToken::Symbol('}') => (AccessKind::Item, '{', '}'),
            _ => break,
        };

        let mut depth = 0i32;
        let mut found_open: Option<usize> = None;

        for i in (0..end - 1).rev() {
            match tokens[i] {
                MToken::Symbol(c) if c == close_ch => depth += 1,
                MToken::Symbol(c) if c == open_ch => {
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
```

---

## Step 4e — Integrate into `parse_expression`

#### Code replacement snippet (parse_expression)

Code to replace (current implementation): 

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
```

New code:

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

---

## Step 5 — Update and add tests

Branch 2 explicitly requires:

* coverage audit updated so Tier‑1 cases stop being opaque
* add focused unit tests: `parse_ident_ref`, `parse_field_access`, `parse_item_access`, `parse_access_chain`, `parse_if_then_else`, `parse_each_expr` 

### Step 5a — Update `m8_m_parser_coverage_audit_tests.rs`

Current file expects all Tier‑1 constructs to be opaque. 
We’ll split into two tests: Tier‑1 structured, Tier‑2 still opaque.

Code to replace (current file): 

```rust
use excel_diff::{MAstKind, canonicalize_m_ast, parse_m_expression};

fn assert_opaque(expr: &str) {
    let mut ast = parse_m_expression(expr).expect("expression should parse into an AST container");
    canonicalize_m_ast(&mut ast);
    match ast.root_kind_for_testing() {
        MAstKind::Opaque { token_count } => {
            assert!(token_count > 0, "opaque token_count must be > 0")
        }
        other => panic!("expected Opaque, got {:?}", other),
    }
}

#[test]
fn coverage_audit_unsupported_constructs_are_opaque() {
    let cases = [
        "Source",
        "#\"Previous Step\"",
        "if true then 1 else 0",
        "each _ + 1",
        "(x) => x",
        "1 + 2",
        "not true",
        "Source[Field]",
        "Source{0}",
        "Source{0}[Content]",
        "x as number",
    ];

    for expr in cases {
        assert_opaque(expr);
    }
}
```

New code:

```rust
use excel_diff::{MAstAccessKind, MAstKind, canonicalize_m_ast, parse_m_expression};

fn parse_kind(expr: &str) -> MAstKind {
    let mut ast = parse_m_expression(expr).expect("expression should parse into an AST container");
    canonicalize_m_ast(&mut ast);
    ast.root_kind_for_testing()
}

fn assert_opaque(expr: &str) {
    match parse_kind(expr) {
        MAstKind::Opaque { token_count } => assert!(token_count > 0),
        other => panic!("expected Opaque, got {:?}", other),
    }
}

fn assert_kind(expr: &str, expected: MAstKind) {
    let got = parse_kind(expr);
    assert_eq!(got, expected);
}

#[test]
fn coverage_audit_tier1_cases_are_structured() {
    assert_kind(
        "Source",
        MAstKind::Ident {
            name: "Source".to_string(),
        },
    );
    assert_kind(
        "#\"Previous Step\"",
        MAstKind::Ident {
            name: "Previous Step".to_string(),
        },
    );

    assert_kind("if true then 1 else 0", MAstKind::If);
    assert_kind("each _ + 1", MAstKind::Each);

    assert_kind(
        "Source[Field]",
        MAstKind::Access {
            kind: MAstAccessKind::Field,
            chain_len: 1,
        },
    );
    assert_kind(
        "Source{0}",
        MAstKind::Access {
            kind: MAstAccessKind::Item,
            chain_len: 1,
        },
    );
    assert_kind(
        "Source{0}[Content]",
        MAstKind::Access {
            kind: MAstAccessKind::Field,
            chain_len: 2,
        },
    );
}

#[test]
fn coverage_audit_tier2_cases_remain_opaque() {
    let cases = ["(x) => x", "1 + 2", "not true", "x as number"];
    for expr in cases {
        assert_opaque(expr);
    }
}
```

---

### Step 5b — Add focused Tier‑1 parser tests

Create a new test file (name doesn’t matter; one reasonable option is `core/tests/m9_m_parser_tier1_tests.rs`).

This complements the coverage audit by making failures highly localized (instead of one big “audit” loop), and matches the plan’s “focused unit tests” list. 

New file to add:

```rust
use excel_diff::{MAstAccessKind, MAstKind, canonicalize_m_ast, parse_m_expression};

fn kind(expr: &str) -> MAstKind {
    let mut ast = parse_m_expression(expr).expect("parse should succeed");
    canonicalize_m_ast(&mut ast);
    ast.root_kind_for_testing()
}

#[test]
fn parse_ident_ref() {
    assert_eq!(
        kind("Source"),
        MAstKind::Ident {
            name: "Source".to_string()
        }
    );

    assert_eq!(
        kind("#\"Previous Step\""),
        MAstKind::Ident {
            name: "Previous Step".to_string()
        }
    );
}

#[test]
fn parse_field_access() {
    assert_eq!(
        kind("Source[Field]"),
        MAstKind::Access {
            kind: MAstAccessKind::Field,
            chain_len: 1
        }
    );
}

#[test]
fn parse_item_access() {
    assert_eq!(
        kind("Source{0}"),
        MAstKind::Access {
            kind: MAstAccessKind::Item,
            chain_len: 1
        }
    );
}

#[test]
fn parse_access_chain() {
    assert_eq!(
        kind("Source{0}[Content]"),
        MAstKind::Access {
            kind: MAstAccessKind::Field,
            chain_len: 2
        }
    );
}

#[test]
fn parse_if_then_else() {
    assert_eq!(kind("if true then 1 else 0"), MAstKind::If);
}

#[test]
fn parse_each_expr() {
    assert_eq!(kind("each _ + 1"), MAstKind::Each);
}

#[test]
fn quoted_identifier_named_then_does_not_confuse_if_parser() {
    let expr = r##"if #"then" then 1 else 0"##;
    assert_eq!(kind(expr), MAstKind::If);
}
```

That last test is specifically to validate the lexer keyword-token choice and prevent regressions where quoted identifiers collide with keyword recognition.

---

### Step 5c — Keep existing canonicalization invariants intact

There are existing tests asserting:

* record field order is canonicalized (semantic equality) 
* list order is not canonicalized (semantic inequality) 
* canonicalization is idempotent 
* boolean/null literal case differences get canonicalized currently (tests refer to “opaque”) 

After Branch 2, `if TRUE then …` will no longer be opaque, but the semantic equality should still hold because `TRUE` parses as `Primitive(Boolean(true))` already. 
You can optionally rename those tests to remove “opaque” from the name, but you shouldn’t need to change their assertions.

---

## Step 6 — Final checklist and “definition of done” mapping

### Required by sprint plan 

* [ ] Identifier refs parse as structured nodes (`MAstKind::Ident`)
* [ ] Access chains parse as structured nodes (`MAstKind::Access`) including chaining
* [ ] `if … then … else …` parses as `MAstKind::If`
* [ ] `each …` parses as `MAstKind::Each`
* [ ] Coverage audit is updated to reflect the Tier‑1 cases are no longer `Opaque`
* [ ] Canonicalization + equality still preserve formatting-only invariants where already tested

### Extra “quality bar” checks that are worth doing because this impacts semantic hashing 

* [ ] Run the semantic diff tests that depend on stable canonical hashes (`FormattingOnly` classification) to ensure new nodes don’t introduce nondeterminism 
* [ ] Confirm that malformed input behavior remains unchanged: only lexical/unbalanced/let-syntax errors return `MParseError`, everything else should stay best-effort and become opaque 

---

## Implementation sequencing (to make this branch land smoothly)

A clean sequence that minimizes “everything breaks at once”:

1. **Lexer keyword tokens + debug mirror**

   * Add tokens
   * Update `tokenize` mapping
   * Ensure existing tokenization tests still pass (e.g., `#date` atomic tokenization) 

2. **AST/MAstKind extensions + root_kind_for_testing**

   * Add new variants
   * Update `root_kind_for_testing`
   * Update any compilation errors in tests due to new `MAstKind` variants

3. **Canonicalization recursion updates**

   * Add match arms for new nodes
   * Keep idempotency intact 

4. **Parser Tier‑1**

   * Add `parse_if_then_else`, `parse_each_expr`, `parse_access_chain`, `parse_ident_ref`
   * Integrate into `parse_expression`

5. **Tests**

   * Update coverage audit
   * Add focused Tier‑1 tests
   * Verify existing semantic diff tests still behave (hash stability)
