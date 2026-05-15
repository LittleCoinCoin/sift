# Register Updater Plugin and Native Menu

**Goal**: Register `tauri-plugin-updater` in the Tauri builder and add a native macOS "Check for Updates…" menu item that emits a `check-for-updates` event to the frontend.
**Pre-conditions**:
- [ ] `deps.md` done — `tauri-plugin-updater = "2"` present in `src-tauri/Cargo.toml`
- [ ] `tauri_config.md` done — `plugins.updater` block present in `src-tauri/tauri.conf.json`
- [ ] Branch created from `main`
**Success Gates**:
- ✅ `cargo build` from `src-tauri/` exits 0 [run]
- ✅ **Sift → Check for Updates…** appears in the macOS native menu bar when the app is running [behavioral]
- ✅ Clicking "Check for Updates…" emits a `check-for-updates` event visible in the frontend (confirmed in the `updater_store.md` integration test) [behavioral]
**References**: [R01 Plan](../../../../.claude/plans/assuming-the-current-project-enchanted-meteor.md) — Rust changes (§ Work breakdown C)

---

## Step 1: Register the updater plugin in the builder chain

**Goal**: Make `tauri-plugin-updater` active at runtime.

**Implementation Logic**: In `src-tauri/src/lib.rs`, inside the `run()` function's `tauri::Builder::default()` chain, add `.plugin(tauri_plugin_updater::Builder::new().build())` as a new `.plugin()` call alongside the existing `tauri_plugin_opener::init()` and `tauri_plugin_dialog::init()` calls. Order among the three plugins does not matter. Add the corresponding `use tauri_plugin_updater` import (or use the fully qualified path inline — follow the style already used by the other two plugins in the file).

**Deliverables**: `src-tauri/src/lib.rs` — new `.plugin(tauri_plugin_updater::Builder::new().build())` call in the `run()` builder chain

**Consistency Checks**: `cargo build --manifest-path src-tauri/Cargo.toml` (expected: PASS)

**Commit**: `feat(updater): register tauri-plugin-updater in builder`

---

## Step 2: Add native macOS "Check for Updates…" menu item

**Goal**: Provide a native macOS menu trigger that fires the update check on demand, enabling manual testing without waiting for the startup delay.

**Implementation Logic**: In the `setup` closure inside `run()` in `src-tauri/src/lib.rs` (where pdfium init currently lives), add a `#[cfg(target_os = "macos")]` block that:
1. Creates a menu item with id `"check_for_updates"` and label `"Check for Updates…"` using `tauri::menu::MenuItemBuilder`.
2. Builds a minimal "Sift" app submenu containing that item plus the standard macOS entries (hide, hide others, show all, separator, quit) using `tauri::menu::SubmenuBuilder`.
3. Builds the top-level `Menu` from that submenu and calls `app.set_menu(menu)`.

Then, outside the `setup` closure (as a chained call on the builder), add `.on_menu_event(|app, event| { ... })` that checks `event.id().0 == "check_for_updates"` and calls `app.emit("check-for-updates", ())`. This emits a Tauri event to all frontend windows, which the updater store listens for.

**Deliverables**: `src-tauri/src/lib.rs` — `#[cfg(target_os = "macos")]` menu setup block in `setup` closure; `.on_menu_event` handler in the builder chain emitting `"check-for-updates"` event

**Consistency Checks**: `cargo build --manifest-path src-tauri/Cargo.toml` (expected: PASS)

**Commit**: `feat(updater): add native macos check-for-updates menu item`
