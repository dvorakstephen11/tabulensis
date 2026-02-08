# Real-World Dataset Program

This directory defines the **real-world dataset program** for Tabulensis.

Everything in this folder is safe to commit: it is **metadata + workload definitions**, not the dataset bytes themselves.

The actual dataset bytes live in a local cache:

- Public pinned datasets: `corpus_public/` (gitignored)
- Private datasets: `corpus_private/` (gitignored; see `docs/robustness_corpus.md`)

## Files

- `registry.yaml`: dataset registry (URLs, sha256 pins, license + attribution)
- `cases.yaml`: workload definitions (diff/open/emit cases over datasets)
- `attributions/`: human-readable notes per dataset id
- `licenses/`: license texts or pointers (only needed if we commit RW1 micro datasets)
- `schemas/`: JSON schemas for Codex structured outputs and (optionally) registry/case validation
- `inspections/`: committed inspection summaries (zip stats, sharedStrings size, etc)

## Quick Start (Public Pinned Datasets)

1) Download and ingest pinned datasets into `corpus_public/`:

```bash
python3 scripts/download_real_world_datasets.py \
  --registry datasets/real_world/registry.yaml \
  --corpus-dir corpus_public \
  --tmp-dir tmp/real_world_downloads
```

2) Inspect and tag datasets (writes `datasets/real_world/inspections/<id>.json`):

```bash
python3 scripts/inspect_dataset.py --registry datasets/real_world/registry.yaml --dataset-id <id> --write
```

3) (Optional) Run a lightweight PII scan:

```bash
python3 scripts/pii_scan_dataset.py --corpus-dir corpus_public --dataset-id <id>
```

Notes:
- This is heuristic; it may flag public contact info (like `name@agency.gov.uk`) as "email".
- You can allowlist known-safe domains:

```bash
python3 scripts/pii_scan_dataset.py --corpus-dir corpus_public --dataset-id <id> --allow-email-domain ons.gov.uk
```

4) Generate the Rust perf tests from `cases.yaml`:

```bash
python3 scripts/generate_real_world_perf_tests.py --cases datasets/real_world/cases.yaml
```

5) Run and export real-world perf metrics:

```bash
python3 scripts/export_real_world_metrics.py \
  --registry datasets/real_world/registry.yaml \
  --cases datasets/real_world/cases.yaml \
  --corpus-dir corpus_public \
  --export-csv benchmarks/latest_real_world.csv
```

## Notes

- By default, the real-world perf tests are `#[ignore]` and gated behind `--features perf-metrics`, consistent with existing perf test patterns.
- Avoid adding raw dataset bytes to git. If we decide to add RW1 "committed micro-real datasets", integrate them into the **fixture generator** pipeline (templates + manifest) so they obey `scripts/check_fixture_references.py`.

## Current Seed Suite (Committed Metadata)

Pinned datasets (see `datasets/real_world/registry.yaml`):
- `rw_ons_lms_v140`, `rw_ons_lms_v141` (ONS "Labour market statistics" previous versions)
- `rw_ons_lx_v5` (ONS "Number surviving at exact age (lx)" previous version)

Cases (see `datasets/real_world/cases.yaml`):
- `rw_ons_lms_v141__diff_vs_v140__streaming_fast`
- `rw_ons_lms_v141__derived_numeric_edits10_seed1__jsonl`
- `rw_ons_lx_v5__identical__streaming_fast`
- `rw_ons_lx_v5__derived_row_block_swap20_seed1__streaming_fast`
