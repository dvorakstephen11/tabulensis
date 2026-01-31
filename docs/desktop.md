# Desktop App

## Run from source

From the repo root:

```bash
cargo run -p desktop_wx
```

Optimized build:

```bash
cargo run -p desktop_wx --profile release-desktop
```

The binary name is `desktop_wx`. After a release build, it lives at:

```
target/<target>/release-desktop/desktop_wx
```

## Packaging

Linux AppImage packaging lives in `scripts/package_desktop_appimage.py`.
