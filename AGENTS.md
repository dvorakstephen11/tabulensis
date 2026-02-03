# Agent Notes

## Common questions

- **Command to open the desktop app (from source):** `cargo run -p desktop_wx`
- **Optimized build:** `cargo run -p desktop_wx --profile release-desktop`
- **More detail:** see `docs/desktop.md` and the “Desktop App (from source)” section in `README.md`.

## Perf Cycle Required For Rust Changes

Whenever Rust code changes (`.rs`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`), run the perf cycle:

1. **Before edits:** `python3 scripts/perf_cycle.py pre`
2. **After edits:** `python3 scripts/perf_cycle.py post --cycle <cycle_id>`

This produces a delta summary in `benchmarks/perf_cycles/<cycle_id>/cycle_delta.md`.

If fixture generation fails in your environment, add `--skip-fixtures`.
