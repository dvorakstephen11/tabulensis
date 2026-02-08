# rw_ons_lms_v141

## Summary
Office for National Statistics (ONS) "Labour market statistics time series" workbook (LMS), previous version `v141`.

Why we use it:
- Realistic OpenXML workbook with a single very large worksheet and time-series numeric data.
- Good for end-to-end parse + streaming diff perf metrics on non-synthetic inputs.

## Source
- Homepage: https://www.ons.gov.uk/employmentandlabourmarket/peopleinwork/employmentandemployeetypes/datasets/labourmarketstatistics/current
- Direct download: https://www.ons.gov.uk/file?uri=%2Femploymentandlabourmarket%2Fpeopleinwork%2Femploymentandemployeetypes%2Fdatasets%2Flabourmarketstatistics%2Fcurrent%2Fprevious%2Fv141%2Flms.xlsx
- Retrieved at: 2026-02-08T16:32:34Z
- SHA256: bd01866794477febd9377da877b79e2570ec11b1ba8c79877720f1146af1245f
- Bytes: 10085898

## License
- License: OGL-UK-3.0 (Open Government Licence v3.0)
- License URL: https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/
- Attribution: Office for National Statistics (ONS)

## Safety / PII
- Heuristic PII scan: pass (2026-02-08)
  - Report: `datasets/real_world/inspections/rw_ons_lms_v141.pii_scan.json`

## Notes / Quirks
- Use alongside `rw_ons_lms_v140` for a natural "real version diff" perf case.
- Also used as the source for the deterministic derived case `rw_ons_lms_v141__derived_numeric_edits10_seed1__jsonl`.
- Inspection summary (2026-02-08): single worksheet; `xl/worksheets/sheet1.xml` is ~125 MB uncompressed.
  - Report: `datasets/real_world/inspections/rw_ons_lms_v141.json`
