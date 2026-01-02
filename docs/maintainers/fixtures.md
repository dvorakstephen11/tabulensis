# Fixture System (Maintainers)

This repo treats fixtures as generated artifacts. Tests and workflows depend on them, but they are not committed.

## Key paths

- `fixtures/generated/`: generated outputs (ignored by git).
- `fixtures/manifest_cli_tests.yaml`: fixtures required for unit/integration tests.
- `fixtures/manifest_perf_e2e.yaml`: large E2E perf fixtures.
- `fixtures/manifest_release_smoke.yaml`: minimal fixtures for release smoke checks.

## Workflow

- Generate fixtures:
  - `generate-fixtures --manifest fixtures/manifest_cli_tests.yaml --force --clean`
- Verify presence + structure:
  - `generate-fixtures --manifest fixtures/manifest_cli_tests.yaml --verify`
- Lock + checksum validation:
  - `generate-fixtures --manifest fixtures/manifest_cli_tests.yaml --write-lock fixtures/manifest_cli_tests.lock.json`
  - `generate-fixtures --manifest fixtures/manifest_cli_tests.yaml --verify-lock fixtures/manifest_cli_tests.lock.json`

## Rules

- Tests and workflows must only reference fixtures produced by their manifest.
- Update the manifest when adding a new fixture reference in tests or CI.
- Use lock files to catch drift; regenerate locks only when fixture definitions change.

## Reference guard

CI runs `scripts/check_fixture_references.py` to ensure tests and workflows only reference fixtures listed in the relevant manifests.
