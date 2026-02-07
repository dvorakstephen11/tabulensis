## Summary

Describe the user-visible and/or semantic impact of this change.

## Testing

- [ ] `python3 scripts/dev_test.py`
- [ ] Any additional targeted tests (describe below)

## Test Review Checklist (New/Changed Tests)

- [ ] Determinism: no hidden state, stable ordering, fixed seeds/timeouts where applicable.
- [ ] Fixtures: referenced outputs exist in the appropriate manifest (prefer `fixtures/manifest_cli_tests.yaml`).
- [ ] Assertions: semantic outcomes, not implementation artifacts.
- [ ] Negative/edge cases included where relevant.
- [ ] Runtime bounds: test remains PR-fast (heavy validation goes to nightly or opt-in jobs).

## Perf / UI Policy (When Relevant)

- [ ] Perf: ran quick/gate or full perf cycle when touching perf-sensitive paths (see `docs/perf_playbook.md` and `AGENTS.md`).
- [ ] UI: if desktop UI changed, ran the canonical UI scenarios (`compare_grid_basic`, `compare_large_mode`, `pbix_no_mashup`) and reviewed artifacts.

## Flake / Quarantine Policy

- [ ] No new flakes introduced. If a test is `#[ignore]`/quarantined: include an issue link, owner, and an expiry plan.

