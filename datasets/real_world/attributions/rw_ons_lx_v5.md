# rw_ons_lx_v5

## Summary
Office for National Statistics (ONS) "Number surviving at exact age (lx), by single year of age" workbook, previous version `v5`.

Why we use it:
- Small, public, permissively licensed workbook for quick "real-ish" smoke/perf cases.
- Good for validating deterministic mutators and JSONL emit workloads without huge outputs.

## Source
- Homepage: https://www.ons.gov.uk/peoplepopulationandcommunity/birthsdeathsandmarriages/lifeexpectancies/datasets/numbersurvivingatexactagelxbysingleyearofage/current
- Direct download: https://www.ons.gov.uk/file?uri=%2Fpeoplepopulationandcommunity%2Fbirthsdeathsandmarriages%2Flifeexpectancies%2Fdatasets%2Fnumbersurvivingatexactagelxbysingleyearofage%2Fcurrent%2Fprevious%2Fv5%2Ftimeseries3yrlx19802021.xlsx
- Retrieved at: 2026-02-08T16:32:34Z
- SHA256: 3a3c2b40e4c9d8d73ff7523fdcbcd6ccec1f11ba289b7c5a8c8d6292959dfb86
- Bytes: 368891

## License
- License: OGL-UK-3.0 (Open Government Licence v3.0)
- License URL: https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/
- Attribution: Office for National Statistics (ONS)

## Safety / PII
- Heuristic PII scan: pass with allowlist (2026-02-08)
  - This workbook contains public contact emails (not personal identifiers).
  - Allowlist used: `ons.gov.uk`, `nationalarchives.gov.uk`
  - Report: `datasets/real_world/inspections/rw_ons_lx_v5.pii_scan.json`

## Notes / Quirks
- Used for:
  - `rw_ons_lx_v5__identical__streaming_fast`
  - `rw_ons_lx_v5__derived_row_block_swap20_seed1__streaming_fast` (derived workload)
- Inspection summary (2026-02-08): 13 sheets; relatively large `xl/styles.xml` for its overall file size.
  - Report: `datasets/real_world/inspections/rw_ons_lx_v5.json`
