# Auto-Updater

## Context
Standalone feature campaign for Sift. Adds in-app update detection and installation via `tauri-plugin-updater` (Tauri v2), wired to the existing bespoke toast system. Produces a persistent "Update available → Install → Restart" notification flow and a native macOS "Check for Updates…" menu item.

## Reference Documents
- [R01 Plan](../../.claude/plans/assuming-the-current-project-enchanted-meteor.md) — Architecture decisions and rationale

## Goal
Ship silent auto-update checking with actionable toasts and a manual trigger from the macOS menu bar.

## Pre-conditions
- [ ] Developer has run `pnpm tauri signer generate` and has the public key ready
- [ ] `TAURI_SIGNING_PRIVATE_KEY` GitHub Repository Secret is set (or will be set before the first signed release)

## Success Gates
- ✅ Launching the app with a lower `version` in `tauri.conf.json` and a local mock endpoint triggers an "Update available" toast after ~4 s
- ✅ Clicking **Install** on the toast starts the download (progress visible)
- ✅ Clicking **Restart Now** on the completion toast relaunches the app
- ✅ **Sift → Check for Updates…** in the macOS menu bar re-triggers the check on demand
- ✅ `pnpm check` and `cargo build` both pass with no new errors
- ✅ CI release job uploads a signed `latest.json` alongside the DMG when `TAURI_SIGNING_PRIVATE_KEY` is present

## Gotchas
- The `/releases/latest/download/latest.json` GitHub endpoint only resolves after a draft is manually published — this is intentional. The updater becomes active only on published releases.
- Testing the full download+install path requires a validly signed `.app.tar.gz` artifact. The notification flow (toasts and buttons) can be verified without it using the local mock endpoint approach described in the plan.
- `dialog: false` must be set in `tauri.conf.json` to suppress Tauri's built-in update dialog in favour of the custom toasts.
- **Release reality (2026-10-03 audit):** the gates above were met in code, not in release. Release builds
  never emitted `.sig` files or `latest.json` (`createUpdaterArtifacts` was missing), every release
  stayed a draft, and the store reported "ready" before installing. The follow-up campaign
  `__roadmap__/update-flow-hardening/` fixes this and ships 0.2.0 as the first auto-updating release.

## Status
```mermaid
graph TD
    deps[Install Plugin Dependencies]:::done
    ci_signing[Add CI Signing Secrets]:::done
    tauri_config[Configure Updater Plugin in tauri.conf.json]:::done
    toast_extension[Extend Toast System for Action Buttons]:::done
    updater_integration[Updater Integration]:::done
    classDef done       fill:#166534,color:#bbf7d0
    classDef inprogress fill:#854d0e,color:#fef08a
    classDef planned    fill:#374151,color:#e5e7eb
    classDef amendment  fill:#1e3a5f,color:#bfdbfe
    classDef blocked    fill:#7f1d1d,color:#fecaca
```

## Nodes
| Node | Type | Status |
|:-----|:-----|:-------|
| `deps.md` | 📄 Leaf Task | ✅ Done |
| `ci_signing.md` | 📄 Leaf Task | ✅ Done |
| `tauri_config.md` | 📄 Leaf Task | ✅ Done |
| `toast_extension.md` | 📄 Leaf Task | ✅ Done |
| `updater_integration/` | 📁 Directory | ✅ Done |

## Amendment Log
| ID | Date | Source | Nodes Added | Rationale |
|:---|:-----|:-------|:------------|:----------|

## Progress
| Node | Branch | Commits | Notes |
|:-----|:-------|:--------|:------|
| `deps.md` | `worktree-agent-a9f831e4ad7fe83a8` | 2 | DEVIATION: ran `pnpm install` (no `--frozen-lockfile`) — expected, lockfile updated correctly |
| `ci_signing.md` | `worktree-agent-aa06fef545b73ae32` | 1 | BLOCKED: behavioral gate (latest.json in release) requires real tag push |
| `tauri_config.md` | `worktree-agent-af432d8aae3a75d52` | 1 | BLOCKED: pubkey is placeholder — developer must run `pnpm tauri signer generate` and replace value |
| `toast_extension.md` | `worktree-agent-a3e902e22787654e4` | 2 | All gates PASS |
