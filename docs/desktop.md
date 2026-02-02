# Desktop App

## Run from source

From the repo root:

```bash
cargo run -p desktop_wx --bin desktop_wx
```

Optimized build:

```bash
cargo run -p desktop_wx --profile release-desktop --bin desktop_wx
```

The binary name is `desktop_wx`. After a release build, it lives at:

```
target/<target>/release-desktop/desktop_wx
```

## UI smoke check

To verify XRC handlers and resources load cleanly (including spacer handlers):

```bash
cargo run -p desktop_wx --bin xrc_smoke
```

For structural validation without running the app, use:

```bash
EXCEL_DIFF_VALIDATE_XRC=1 cargo run -p desktop_wx --bin desktop_wx
```

## Web UI toggle

By default the desktop app now uses the native (XRC) UI. To force the WebView UI:

```bash
EXCEL_DIFF_USE_WEBVIEW=1 cargo run -p desktop_wx --bin desktop_wx
```

## Architecture (UI)

- `core/`: deterministic diff logic.
- `desktop/backend/`: engine/workflow orchestration + state.
- `ui_payload/`: shared DTOs/protocol used by desktop + web renderers.

## Debug + environment tips (Linux)

- To suppress noisy GTK/GSettings warnings during dev (default in debug builds):

```bash
EXCEL_DIFF_SUPPRESS_GTK_WARNINGS=1
```

- Opt out:

```bash
EXCEL_DIFF_SUPPRESS_GTK_WARNINGS=0
```

When suppression is enabled, GTK/GDK log handlers are muted and app stdio is redirected to keep the console clean.

- Cursor theme warnings: install a complete cursor theme, or set

```bash
EXCEL_DIFF_CURSOR_THEME=Adwaita
EXCEL_DIFF_CURSOR_SIZE=24
```

If you want a default fallback without setting XCURSOR_THEME yourself:

```bash
EXCEL_DIFF_FORCE_CURSOR_THEME=1
```

- Overlay scrollbar warnings: disable overlay scrollbars if your GTK stack reports negative sizes

```bash
EXCEL_DIFF_DISABLE_OVERLAY_SCROLLBARS=1
```

## License checks (dev)

By default, debug builds skip license enforcement so diffs work out of the box.
To force license checks:

```bash
EXCEL_DIFF_REQUIRE_LICENSE=1
```

To explicitly skip in a non-debug build:

```bash
EXCEL_DIFF_SKIP_LICENSE=1
```

## Window behavior

Start maximized (default on first run):

```bash
EXCEL_DIFF_START_MAXIMIZED=1
```

Disable auto-maximize:

```bash
EXCEL_DIFF_START_MAXIMIZED=0
```

## Local state

- UI layout + last selections are stored at `ui_state.json` in the app data directory.
- Crash diagnostics are appended to `crash.log` in the same directory.

## Packaging

Linux AppImage packaging lives in `scripts/package_desktop_appimage.py`.
