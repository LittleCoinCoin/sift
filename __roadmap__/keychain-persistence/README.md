# Keychain Persistence

## Context
`keyring = "3"` is built without a platform feature, so every API key saved in Settings goes to keyring's in-memory mock and is lost (`__reports__/api_key_storage/00-keyring_mock_backend_v0.md`). The signing measurement (`01-keychain_signing_measurement_v0.md`) settled the design.
- With `apple-native` on, keys persist.
- After each update, macOS asks for the login keychain password once per stored key ("Always Allow"), under ad-hoc and self-signed signing alike.
- The maintainer accepts that prompt. Self-signing is dropped, and a paid Developer ID is out of scope.
This campaign ships the real backend in v0.2.1.

## Goal
API keys saved in Settings survive relaunches and updates, the Keychain dialog never freezes the window, and v0.2.1 ships it.

## Pre-conditions
- [x] Integration branch `milestone/keychain-persistence` cut from `main` at `57c225b`
- [x] Environment contract measured in a fresh hand-made worktree: `pnpm install --frozen-lockfile && pnpm build && (cd src-tauri && cargo test --lib)` green, with the cargo paths inside that worktree (2026-10-06, experiment worktree preflight: 43 passed, target dir `experiment-keychain-signing/src-tauri/target`)

## Success Gates
- ✅ `(cd src-tauri && cargo test --lib)` passes on the integration tip, including `backend_persists_until_delete` [run]
- ✅ `(cd src-tauri && cargo tree -e features -i keyring)` lists `apple-native` [run]
- ✅ `git diff 57c225b -- src-tauri/Cargo.lock | grep '^+name = '` prints exactly `+name = "security-framework"` (the iOS-only 2.11.1) [run]
- ✅ `git diff --exit-code 57c225b -- pnpm-lock.yaml` exits 0 [run]
- ✅ `pnpm e2e:native` run by the maintainer on the integration tip exits 0, with `KEYCHAIN_AFTER_UPDATE=` and `PASS  ipc stays responsive while the keychain read is pending` in the transcript [behavioral]
- ⬜ `uvx --from commitizen==4.19.1 cz bump --get-next` prints `0.2.1` before the release PR [run]
- ⬜ `curl -fsSL https://github.com/LittleCoinCoin/sift/releases/latest/download/latest.json` returns version `0.2.1` [run]

## Gotchas
- **Agents cannot run `pnpm e2e:native`.** The auto-mode classifier refused it (report 01, defect 4). The e2e leaf's live gate is run by the maintainer at L1 close. Implementers gate on `bash -n` and on the driver's typecheck and build.
- **A Keychain dialog during the E2E is expected.** At 0.2.0 the read blocks on a login-password dialog. Nobody types into it during a run; the script kills the app after about 26 s.
- **One new lockfile package is expected:** `security-framework 2.11.1`, keyring's iOS dependency. macOS targets do not build it.
- **Verifier leaves:** `apple_native_backend` (persisted contract, IPC threading) and `e2e_keychain_assertions` (measurement semantics, isolation guard). `release_v0_2_1` is coordinator-direct and gated by the maintainer.
- **Ownership:**
  - `apple_native_backend` owns `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` and `src-tauri/src/keyring_store.rs`.
  - `e2e_keychain_assertions` owns `scripts/e2e-updater.sh` and `src/lib/e2e/updater-driver.ts`.
  - `release_v0_2_1` owns `CHANGELOG.md`, `__reports__/api_key_storage/README.md`, the 00 report's status, and the Reminders.
- **Merge commits:** subject `Merge task/<leaf> into milestone/keychain-persistence`. The body is WHY prose and never starts a line with `type(scope):`.
- **Handoff (2026-10-06, before L1 dispatch).** The coordinator works in worktree `table-letter-s-to-p-abd036` on `milestone/keychain-persistence`.
  - Next: hand-make worktrees `task/apple_native_backend` and `task/e2e_keychain_assertions` from the integration tip, and measure the environment contract in one of them.
  - Then dispatch both implementers in parallel (Sonnet, `model: "sonnet"`), with briefs carrying the toolchain (local pnpm, cargo, uv; no mamba), the colgrep `index_build` and `paths=[<worktree>]` note, and "report spec defects, don't implement them".
  - Verifier attack targets:
    - backend: does the persistence test really fail on the mock? Does `command(async)` change any IPC contract or error string?
    - e2e: can `blocked` be misread as `ok` or `error`? Does `ipc alive` really fail if the main thread is held? Is the isolation guard still sound?
  - After both merge, the maintainer runs `pnpm e2e:native`.
- **Experiment branch** `experiment/keychain-signing` (worktree `../experiment-keychain-signing`) is evidence only. It is never merged, and it is removed at campaign close.

## Status
```mermaid
graph TD
    apple_native_backend[Apple Native Backend]:::done
    e2e_keychain_assertions[E2E Keychain Assertions]:::done
    next[Release v0.2.1]:::inprogress
    classDef done       fill:#166534,color:#bbf7d0
    classDef inprogress fill:#854d0e,color:#fef08a
    classDef planned    fill:#374151,color:#e5e7eb
    classDef amendment  fill:#1e3a5f,color:#bfdbfe
    classDef blocked    fill:#7f1d1d,color:#fecaca
```

## Nodes
| Node | Type | Status |
|:-----|:-----|:-------|
| `apple_native_backend.md` | 📄 Leaf Task | ✅ Done |
| `e2e_keychain_assertions.md` | 📄 Leaf Task | ✅ Done |
| `next/` | 📁 Directory | 🔄 In Progress |

## Amendment Log
| ID | Date | Source | Nodes Added | Rationale |
|:---|:-----|:-------|:------------|:----------|

## Progress
| Node | Branch | Commits | Notes |
|:-----|:-------|:--------|:------|
