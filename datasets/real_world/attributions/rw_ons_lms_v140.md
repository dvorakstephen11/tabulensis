# rw_ons_lms_v140

## Summary
Office for National Statistics (ONS) "Labour market statistics time series" workbook (LMS), previous version `v140`.

Why we use it:
- Paired with `rw_ons_lms_v141` to form a natural real-world diff pair.
- Provides realistic structure (single large worksheet) and non-synthetic diffs for streaming diff performance metrics.

## Source
- Homepage: https://www.ons.gov.uk/employmentandlabourmarket/peopleinwork/employmentandemployeetypes/datasets/labourmarketstatistics/current
- Direct download: https://www.ons.gov.uk/file?uri=%2Femploymentandlabourmarket%2Fpeopleinwork%2Femploymentandemployeetypes%2Fdatasets%2Flabourmarketstatistics%2Fcurrent%2Fprevious%2Fv140%2Flms.xlsx
- Retrieved at: 2026-02-08T16:32:34Z
- SHA256: 3c5e6db6a4ec1c7a22605c8733729b50ca1beba9f3bf1f4ac057459919f47ed9
- Bytes: 10077183

## License
- License: OGL-UK-3.0 (Open Government Licence v3.0)
- License URL: https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/
- Attribution: Office for National Statistics (ONS)

## Safety / PII
- Heuristic PII scan: pass (2026-02-08)
  - Report: `datasets/real_world/inspections/rw_ons_lms_v140.pii_scan.json`

## Notes / Quirks
- Intended pairing: `rw_ons_lms_v140` (A) vs `rw_ons_lms_v141` (B).
- Inspection summary (2026-02-08): single worksheet; `xl/worksheets/sheet1.xml` is ~125 MB uncompressed.
  - Report: `datasets/real_world/inspections/rw_ons_lms_v140.json`
