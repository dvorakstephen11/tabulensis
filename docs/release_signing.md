# Release Signing and Notarization

This repo includes packaging scripts for the CLI and desktop builds. Signing and
notarization are expected to run in CI with secrets supplied at runtime.

## Decision Summary (2026-02-09)

- Windows: Azure Artifact Signing (Trusted Signing) via GitHub OIDC on GitHub-hosted Windows runners.
- macOS: Apple Developer ID signing + notarization (`notarytool`) + stapling.

## Current State (Repo Reality, 2026-02-09)

- Canonical release workflow: `.github/workflows/release.yml` (tag `v*`).
- macOS:
  - Universal `tabulensis` binary is ad-hoc signed in CI (`codesign -s - ...`).
  - Notarization is not wired yet.
- Windows:
  - No Authenticode signing step is wired yet.

This doc describes the intended target state; treat it as a TODO until CI wiring exists.

## macOS

Target:
- Apple Developer ID certificate.
- Hardened runtime signing (`codesign --options runtime --timestamp ...`).
- Notarization with `xcrun notarytool`.
- Staple ticket to the distributed artifact.

Concrete checklist + setup steps:
- `docs/macos_signing_notarization.md`

## Windows

Target:
- Sign Windows artifacts via Azure Artifact Signing (Trusted Signing).
- Verify signatures with `signtool verify`.

Notes:
- The Azure signing GitHub Action runs on Windows runners, so the signing step must live in a `runs-on: windows-*` job.
- Sign before packaging and before checksums (signing mutates the file bytes).
- Typical wiring: `azure/login` (OIDC) + `azure/artifact-signing-action@v1` (sign) + `signtool verify` (verify).

## CI Key Handling (Chosen)

Decision: `docs/meta/results/decision_register.md` (`DR-0020`).

Chosen approach (Windows):
- Azure Artifact Signing (Trusted Signing) using GitHub OIDC auth.
- Keep the signing job gated behind a GitHub Environment with required reviewers.
- Only run signing on tag pushes (`refs/tags/v*`) and when `dry_run != true`.

Suggested CI config (GitHub Environment):
- Non-sensitive (vars are fine):
  - `AZURE_TENANT_ID`
  - `AZURE_SUBSCRIPTION_ID`
  - `AZURE_CLIENT_ID` (app registration client id used for OIDC)
  - `AZURE_ARTIFACT_SIGNING_ENDPOINT` (example: `https://eus.codesigning.azure.net/`)
  - `AZURE_ARTIFACT_SIGNING_ACCOUNT_NAME`
  - `AZURE_ARTIFACT_SIGNING_CERT_PROFILE_NAME`
- Secrets:
  - None required for OIDC itself (the point is to avoid long-lived credentials).
  - If you cannot use OIDC, fall back to Azure credentials as a secret (service principal), but treat that as a last resort.

Guardrails:
- Signing job must request `permissions: id-token: write` for OIDC.
- After signing, run `signtool verify /pa /v <exe>` as a smoke check.

## Suggested CI environment variables

- macOS: see `docs/macos_signing_notarization.md` (recommended secret/var names and CI wiring).
- Windows (Azure Artifact Signing):
  - `AZURE_TENANT_ID`
  - `AZURE_SUBSCRIPTION_ID`
  - `AZURE_CLIENT_ID`
  - `AZURE_ARTIFACT_SIGNING_ENDPOINT`
  - `AZURE_ARTIFACT_SIGNING_ACCOUNT_NAME`
  - `AZURE_ARTIFACT_SIGNING_CERT_PROFILE_NAME`

Add these to your CI secrets manager and wire them into a release workflow.
