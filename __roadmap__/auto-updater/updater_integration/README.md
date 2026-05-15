# Updater Integration

## Context
Second phase of the auto-updater campaign. Runs after all depth-0 leaves are complete (packages installed, config ready, toast system extended). Produces the Rust plugin registration + native menu on one branch, and the TypeScript updater store + App.svelte wiring on another — both branches are independent and can be worked in parallel.

## Goal
Wire the updater plugin into both the Rust backend (plugin + menu) and the Svelte frontend (store + App mount).

## Pre-conditions
- [ ] `deps.md` done — `tauri-plugin-updater` in Cargo.toml, `@tauri-apps/plugin-updater` and `@tauri-apps/plugin-process` in package.json
- [ ] `toast_extension.md` done — extended `Toast` type with `action?` and `persistent?` fields, helper exports available from `log.ts`
- [ ] `tauri_config.md` done — `plugins.updater` block present in `tauri.conf.json`

## Success Gates
- ✅ `cargo build` passes with `tauri_plugin_updater` registered in the builder chain
- ✅ **Sift → Check for Updates…** appears in the macOS native menu bar and emits the `check-for-updates` event
- ✅ `pnpm check` passes with no type errors on the new store
- ✅ End-to-end local test (mock endpoint + lowered version): "Update available" toast appears ~4 s after launch with an **Install** action button

## Status
```mermaid
graph TD
    rust_plugin[Register Updater Plugin and Native Menu]:::planned
    updater_store[Implement Updater Store and Wire into App]:::planned
    classDef done       fill:#166534,color:#bbf7d0
    classDef inprogress fill:#854d0e,color:#fef08a
    classDef planned    fill:#374151,color:#e5e7eb
    classDef amendment  fill:#1e3a5f,color:#bfdbfe
    classDef blocked    fill:#7f1d1d,color:#fecaca
```

## Nodes
| Node | Type | Status |
|:-----|:-----|:-------|
| `rust_plugin.md` | 📄 Leaf Task | ⬜ Planned |
| `updater_store.md` | 📄 Leaf Task | ⬜ Planned |

## Amendment Log
| ID | Date | Source | Nodes Added | Rationale |
|:---|:-----|:-------|:------------|:----------|

## Progress
| Node | Branch | Commits | Notes |
|:-----|:-------|:--------|:------|
