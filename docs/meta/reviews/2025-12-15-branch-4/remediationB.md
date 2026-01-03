## Practical migration recipe (the 90% mechanical fixes)

Below are the high-frequency “replace this pattern with that pattern” changes that will knock out most of the errors.

### 1) Cell insertion

Old pattern (no longer valid):

```rust
grid.insert(Cell {
    row,
    col,
    address: CellAddress::from_indices(row, col),
    value: Some(CellValue::Number(*value)),
    formula: None,
});
```

New pattern:

```rust
grid.insert_cell(row, col, Some(CellValue::Number(*value)), None);
```

### 2) Sheet names / IDs in tests

Old pattern:

```rust
.find(|s| s.name == "Data")
```

New pattern (recommended, since you already have `sid()` in test common):

```rust
.find(|s| s.name == sid("Data"))
```

### 3) Text values and formulas

Old pattern:

```rust
Some(CellValue::Text("old".into()))
```

New pattern:

```rust
Some(CellValue::Text(sid("old")))
```

Old formula pattern:

```rust
formula: Some("A1+1".to_string()),
```

New formula pattern:

```rust
formula: Some(sid("A1+1")),
```

### 4) Database-mode diff call sites

Old pattern:

```rust
let report = diff_grids_database_mode(&grid_a, &grid_b, &[0], &excel_diff::DiffConfig::default());
```

New pattern (minimal change):

```rust
let report = with_default_session(|session| {
    diff_grids_database_mode(&grid_a, &grid_b, &[0], &mut session.strings, &excel_diff::DiffConfig::default())
});
```

If you want to avoid threading pool through every test call, I’d strongly consider adding a crate-root wrapper for database mode (mirroring what you already did for `diff_workbooks`). Right now, the API shape is inconsistent, which is why the tests are fighting you.

## One architectural note I would fix before calling Branch 4 “complete”

### String table cloning vs moving

In `try_diff_workbooks`, you’re cloning the pool’s strings into the report (`pool.strings().to_vec()`). That’s functionally correct, but it partially undercuts the memory goal on large inputs (you pay for the string table twice at report construction time).

The spec’s direction is: when you *own* the pool lifetime (CLI path; JSON output path), prefer **moving** the strings into the report. You already have `StringPool::into_strings(self) -> Vec<String>`, so the missing piece is shaping the top-level “diff from files” pipeline so it can consume the session/pool at the end.

## About the benchmark results you attached

`benchmark_results.json` is for `git_branch: "2025-12-13-branch-3"` (not this Branch 4 code), so it doesn’t validate the new memory/streaming changes yet.

## Missing “branch completeness” signals

From the context you shared, I see a perf workflow, but I don’t see the “WASM smoke build gate” pieces that your Branch 4 plan calls out (workflow + minimal wasm binary). If Branch 4 includes a WASM readiness requirement, that’s still an open checklist item.
