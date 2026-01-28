# Auto-Update Strategy

Tabulensis currently ships manual downloads. A lightweight auto-update plan:

1. Publish a signed JSON manifest per release with version + download URLs.
2. CLI: add `tabulensis update` to compare versions and download the correct artifact.
3. Desktop: add a startup check that prompts the user when a newer version is available.
4. Store the manifest URL in config and allow overrides for enterprise installs.

Implementation is intentionally deferred until the signing/notarization workflow
is stable for each platform.
