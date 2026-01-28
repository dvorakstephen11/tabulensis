# Release Signing and Notarization

This repo includes packaging scripts for the CLI and desktop builds. Signing and
notarization are expected to run in CI with secrets supplied at runtime.

## macOS

- Apple Developer ID certificate
- Notarization with `xcrun notarytool`
- Staple ticket to the binary/app bundle

## Windows

- Authenticode code-signing certificate (OV)
- Sign EXE/MSI artifacts with `signtool`

## Suggested CI environment variables

- `MACOS_SIGNING_IDENTITY`
- `MACOS_NOTARY_KEY_ID`
- `MACOS_NOTARY_ISSUER_ID`
- `MACOS_NOTARY_KEY_PATH`
- `WINDOWS_SIGNING_CERT_PATH`
- `WINDOWS_SIGNING_CERT_PASSWORD`

Add these to your CI secrets manager and wire them into a release workflow.
