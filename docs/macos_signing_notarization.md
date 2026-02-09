# macOS Signing + Notarization (Developer ID)

Goal: ship macOS artifacts that launch without scary Gatekeeper warnings for normal users.

This doc is written for Tabulensis CLI artifacts today, but the same Apple primitives apply to a future `.app`/DMG desktop release.

## Cost + Time

Money:
- Apple Developer Program membership: `$99 USD / year` (individual or organization).

Time (typical; varies with your org/legal setup):
- Individual enrollment: often same-day if your Apple ID is in good standing.
- Organization enrollment: expect a few business days, sometimes 1-2 weeks, depending on D-U-N-S/legal verification and Apple callbacks.
- Notarization per build: usually minutes; budget `< 1 hour` worst-case for planning.

## One-Time Setup Checklist

Account + identity:
- [ ] Enroll in the Apple Developer Program (individual or organization).
- [ ] If enrolling as an organization: confirm your legal entity info matches D-U-N-S, and wait for Apple verification to complete.
- [ ] Ensure the Apple Developer account holder can access Certificates/Identifiers/Profiles.

Certificates (Developer ID):
- [ ] Create a **Developer ID Application** certificate (used to sign the binary or `.app`).
- [ ] If you plan to ship a signed `.pkg` installer: also create **Developer ID Installer**.
- [ ] On a secure Mac, export the Developer ID Application cert + private key as a `.p12` (for CI import).
  - Treat this like a high-value secret (it contains your private key).

Notarization credentials (App Store Connect):
- [ ] Create an App Store Connect API key used for notarization.
  - Download the `.p8` key (you only get it once).
  - Record `key_id` and `issuer_id`.

## Recommended CI Secret/Var Names

These names match the existing conventions in `docs/meta/results/2026-02-09_implementation_plan_pro_payload_and_release_signing.md` and keep the workflow wiring straightforward.

macOS signing (high-value secrets):
- `MACOS_CERT_P12_B64` (base64 of Developer ID Application `.p12`)
- `MACOS_CERT_PASSWORD` (password for the `.p12`)
- `MACOS_SIGNING_IDENTITY` (codesign identity string; example: `Developer ID Application: Your Company (TEAMID)`)

macOS notarization (high-value secrets):
- `MACOS_NOTARY_KEY_B64` (base64 of the App Store Connect API key `.p8`)
- `MACOS_NOTARY_KEY_ID`
- `MACOS_NOTARY_ISSUER_ID`

Guardrail suggestion:
- Store these in a GitHub Environment (example: `release-signing`) with required reviewers, and only run signing/notarization on tag builds.

## Artifact Format Decision (CLI)

Apple notarization workflows commonly operate on top-level artifacts like:
- `.zip`
- `.dmg`
- `.pkg`

The repo currently ships macOS CLI artifacts as `.tar.gz`. For notarization, plan to do one of:
- Option A (recommended): switch macOS release artifacts to `.zip`.
- Option B: ship both `.zip` (notarized) and `.tar.gz` (legacy), but only advertise the `.zip`.

## Per-Release Checklist (CI)

Keychain import:
- [ ] Create an ephemeral keychain for CI signing.
- [ ] Decode `MACOS_CERT_P12_B64` to a temp file and import it into the ephemeral keychain.
- [ ] Configure keychain search path/unlock so `codesign` can find the identity.

Sign:
- [ ] Build the final binary you will distribute.
- [ ] Sign with hardened runtime + timestamp:
  - `codesign --force --options runtime --timestamp --sign "$MACOS_SIGNING_IDENTITY" tabulensis`
- [ ] Verify signature:
  - `codesign --verify --strict --verbose=2 tabulensis`

Package (for notarization):
- [ ] Create a `.zip` containing the signed artifact(s) you intend users to run.
  - For CLI, a common pattern is a zip containing `tabulensis` + `README.md` + licenses.

Notarize:
- [ ] Decode `MACOS_NOTARY_KEY_B64` to `AuthKey.p8` (temp file).
- [ ] Submit and wait:
  - `xcrun notarytool submit <artifact.zip> --key AuthKey.p8 --key-id "$MACOS_NOTARY_KEY_ID" --issuer "$MACOS_NOTARY_ISSUER_ID" --wait`
- [ ] If notarization fails: capture the `notarytool` log output as a CI artifact for debugging.

Staple:
- [ ] Staple the ticket to the distributable:
  - `xcrun stapler staple <artifact.zip>`

Validate the user experience:
- [ ] Gatekeeper assessment:
  - `spctl -a -vvv <artifact.zip>` (or the extracted binary, depending on packaging)

Checksums:
- [ ] Compute SHA256 after signing/notarization/stapling (these steps change bytes).

## Notes For A Future Desktop `.app` / DMG

- The `.app` bundle must be signed (often with additional entitlements depending on capabilities).
- DMG is typically notarized as the top-level artifact; you sign the app inside, then notarize/staple the DMG.
- If you ship a `.pkg` installer, you generally sign it with Developer ID Installer.

