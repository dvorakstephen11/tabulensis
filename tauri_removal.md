# Tauri removal plan

## Goals
- Remove the Tauri desktop shell code and all CLI/documentation references to it.
- Keep the wxDragon desktop app (`desktop/wx`) and backend (`desktop/backend`) as the only desktop UI path.

## Plan
1. Confirm scope and keepers.
   - Keep `desktop/wx` and `desktop/backend`.
   - Remove the entire Tauri shell under `desktop/src-tauri`.

2. Remove the Tauri crate and workspace wiring.
   - Delete `desktop/src-tauri/` (Cargo.toml, build.rs, tauri.conf.json, icons, gen schemas, src).
   - Remove `desktop/src-tauri` from `Cargo.toml` workspace members.
   - Regenerate `Cargo.lock` by running a build/check for remaining crates so `tauri*` crates disappear.

3. Remove the Tauri web bridge.
   - Delete `web/native_diff_client.js` (Tauri bridge).
   - Simplify `web/platform.js` to always use `createDiffWorkerClient` and drop `isDesktop()` + native dialog calls.
   - Remove any remaining `__TAURI__` checks or Tauri-specific imports in `web/`.

4. Update scripts, tooling, and CI hooks.
   - Remove `desktop/src-tauri` from `scripts/verify_release_versions.py` defaults.
   - Search scripts/CI for `excel_diff_desktop` or `src-tauri` and remove those commands.

5. Remove CLI-facing references in docs and guides.
   - Update or remove any CLI commands mentioning the Tauri package (e.g., `cargo check -p excel_diff_desktop`).
   - Update user-facing docs to drop Tauri mentions:
     - `README.md`
     - `REPO_STATE.md`
     - `docs/host_parity.md`
     - `docs/maintainers/entrypoints.md`
   - Retire or rewrite Tauri-specific design docs:
     - `docs/rust_docs/ui_refactor_to_desktop_executable.md`
     - `docs/rust_docs/workstream_master_doc.md`

6. Validate.
   - `rg -n "tauri|src-tauri|excel_diff_desktop|__TAURI__" -S` should return none (or only agreed archival files).
   - `cargo check --workspace` (and optionally `cargo test --workspace`) to ensure the workspace builds without the Tauri crate.
