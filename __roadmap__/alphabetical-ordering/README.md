# Alphabetical Ordering

## Context
Standalone fix for non-deterministic display order in the Receipt Viewer. The document tree file view and the extraction fields panel both render in arbitrary order on every mount because the Rust backend returns HashMap values (unordered) and the Svelte frontend never sorts them before display.

## Goal
Enforce stable alphabetical ordering for both the document tree and extraction fields across the full stack.

## Pre-conditions
- [ ] Working directory clean on the `claude/awesome-jennings-238eb8` branch

## Success Gates
- ✅ `cargo test` passes in `src-tauri/` including the new sort-order test [run]
- ✅ Document tree (tree view and flat view) shows files sorted alphabetically by path on every mount [behavioral]
- ✅ Subdirectories within the tree are sorted alphabetically [behavioral]
- ✅ Extraction fields panel shows field keys in alphabetical order for any processed receipt [behavioral]
- ✅ Field order does not change while the user is typing in a field [behavioral]

## Status
```mermaid
graph TD
    sort_rust_backend[Sort Rust Backend]:::planned
    sort_svelte_frontend[Sort Svelte Frontend]:::planned
    classDef done       fill:#166534,color:#bbf7d0
    classDef inprogress fill:#854d0e,color:#fef08a
    classDef planned    fill:#374151,color:#e5e7eb
    classDef amendment  fill:#1e3a5f,color:#bfdbfe
    classDef blocked    fill:#7f1d1d,color:#fecaca
```

## Nodes
| Node | Type | Status |
|:-----|:-----|:-------|
| `sort_rust_backend.md` | 📄 Leaf Task | ⬜ Planned |
| `sort_svelte_frontend.md` | 📄 Leaf Task | ⬜ Planned |

## Amendment Log
| ID | Date | Source | Nodes Added | Rationale |
|:---|:-----|:-------|:------------|:----------|

## Progress
| Node | Branch | Commits | Notes |
|:-----|:-------|:--------|:------|
