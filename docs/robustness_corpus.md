# Robustness Corpus

This document defines how the project grows and curates the "weird file" corpus used for
fuzzing and robustness regression tests.

## Corpus tiers

Tier 1: Committed deterministic fixtures
- Source: `fixtures/manifest_cli_tests.yaml` outputs.
- Purpose: deterministic CI coverage and regression tests.
- Rule: small, public, reproducible.

Tier 2: Committed fuzz seed corpus
- Source: a curated subset of Tier 1 copied into `core/fuzz/corpus/*`.
- Purpose: fast, deterministic seeds for `cargo fuzz`.
- Rule: small and curated; updated via `scripts/seed_fuzz_corpus.py`.

Tier 3: Private enterprise corpus (not committed)
- Source: local "real-world" files that cannot be shared.
- Purpose: high-value fuzzing and repro discovery.
- Rule: stored only in hashed form, never committed.

## Naming and privacy

- Use hash-based filenames for private corpora.
- Do not store original filenames, folder names, or customer identifiers.
- Use regression filenames like `reg_<area>_<shortdesc>_<sha8>.<ext>`.

## Seeding fuzz corpora from fixtures

1) Generate fixtures (if needed):

```bash
python fixtures/src/generate.py --manifest fixtures/manifest_cli_tests.yaml --output-dir fixtures/generated
```

2) Seed fuzz corpora from a curated list:

```bash
python scripts/seed_fuzz_corpus.py --config core/fuzz/seed_fixtures.yaml
```

The seeding script copies container files into `core/fuzz/corpus/<target>` and extracts
DataMashup bytes for `fuzz_datamashup_parse`.

## Private corpus ingestion

Private corpora live under `corpus_private/` (gitignored). Use the ingestion helper to
store hashed copies and a local metadata index:

```bash
python scripts/ingest_private_corpus.py --input-dir <path> --source-tag <label>
```

## Local robustness smoke tests

Run the Phase 9 regression sweep:

```bash
cargo test -p excel_diff robustness_regressions
```

These tests are deterministic and use hardening limits to avoid hangs.
