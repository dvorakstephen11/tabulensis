# Git Integration

[Docs index](index.md)

Git can't diff binary `.xlsx` / `.xlsm` directly. Tabulensis supports two practical integrations:

1. **Textconv** (best for `git diff`): converts a single workbook into stable text.
2. **Difftool** (best for workbook-vs-workbook): compares two versions and emits a unified diff.

## 1) Textconv (recommended for `git diff`)

Add file patterns to `.gitattributes` (in your repo):

```gitattributes
*.xlsx diff=xlsx
*.xlsm diff=xlsx
```

Add a diff driver to `~/.gitconfig` (or `.git/config`):

```gitconfig
[diff "xlsx"]
    textconv = tabulensis info
    cachetextconv = true
    binary = true
```

Now `git diff` will show a stable text view (workbook name, sheets, dimensions, and optionally queries if you run `tabulensis info --queries` manually).

## 2) True diff via difftool (recommended for workbook-vs-workbook)

Add this to `~/.gitconfig`:

```gitconfig
[difftool "tabulensis"]
    cmd = tabulensis diff --git-diff "$LOCAL" "$REMOTE"
```

Then run:

```bash
git difftool --tool=tabulensis -- path/to/file.xlsx
```

## 3) PBIP / PBIR / TMDL (text-native) normalization

PBIP projects store report + model artifacts as files (for source control), but raw diffs can be noisy
(JSON ordering, GUID churn, whitespace).

Tabulensis can normalize PBIR (`.pbir`) and TMDL (`.tmdl`) per-file for a stable `git diff`.

Add file patterns to `.gitattributes`:

```gitattributes
*.pbir diff=pbip
*.tmdl diff=pbip
```

Add a diff driver to `~/.gitconfig` (or `.git/config`):

```gitconfig
[diff "pbip"]
    textconv = tabulensis pbip normalize --profile balanced
    cachetextconv = true
```

Now `git diff` will show canonicalized PBIR JSON (sorted keys + GUID normalization per profile) and
lexically-normalized TMDL.

For a one-shot PR summary across two PBIP folders (document + entity rollups), use:

```bash
tabulensis pbip diff --markdown <OLD_DIR> <NEW_DIR>
```

## Notes / edge cases

- `--git-diff` cannot be combined with `--format json` or `--format jsonl`.
- `.xlsx` files are binary; textconv is useful even if you don't want a full workbook-vs-workbook diff.
- For large workbooks, consider using a wrapper script/alias that adds `--max-memory` and `--timeout` and reference that wrapper from your `textconv`/`difftool` config.

