# Fuzz Triage Guide

This guide explains how to turn fuzz findings into minimized regressions and tests.

## Severity rubric

- P0: crash, panic, UB, OOM, timeout/hang
- P1: deterministic misparse (invariant violation)
- P2: nondeterminism (self-diff not empty)
- P3: spec deviations that do not affect stability

## Triage workflow

1) Reproduce and minimize

```bash
python scripts/fuzz_triage.py --target <target> --artifact <path>
```

2) Convert to a regression fixture

```bash
python scripts/add_regression_fixture.py \
  --artifact <minimized> \
  --type xlsx \
  --area workbook \
  --description "short summary" \
  --expectation error \
  --error-code EXDIFF_PKG_003
```

3) Regenerate fixtures and update the lock file

```bash
python fixtures/src/generate.py --manifest fixtures/manifest_cli_tests.yaml --output-dir fixtures/generated --write-lock fixtures/manifest_cli_tests.lock.json --force
```

4) Add a corpus seed

```bash
python scripts/seed_fuzz_corpus.py --config core/fuzz/seed_fixtures.yaml --overwrite
```

5) Run the regression suite

```bash
cargo test -p excel_diff robustness_regressions
```

## Notes

- Keep regression fixtures small and deterministic.
- Prefer container files (`.xlsx`, `.pbix`) unless a DataMashup-only `.bin` is the smallest repro.
- Always include the error code in the expectations file for error cases.
