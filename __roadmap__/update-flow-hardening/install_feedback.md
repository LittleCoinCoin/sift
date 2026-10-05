# Install Feedback

**Goal**: Make the update toasts tell the truth: one updater toast at a time, an explicit install phase, "ready" only after the install succeeded, and readable messages for every install failure.
**Pre-conditions**:
- [ ] Working on `task/install_feedback`, branched from the integration tip (which includes the coordinator's typecheck baseline, so `svelte-check` actually checks files)
**Success Gates**:
- ⬜ `pnpm check` (svelte-check) reports 0 errors, and `pnpm build` passes [run]
- ⬜ `grep -c isFinished src/lib/stores/updater.svelte.ts` prints `0` [static]
- ⬜ `node --test src/lib/stores/updater-errors.test.ts` passes. It covers the report-01 strings (`-128` / `User canceled`, `PermissionDenied (os error 1)`), the read-only volume string (`Read-only file system (os error 30)`), a signature-verification failure string, and an unknown string passing through unchanged [run]
- ⬜ `git diff --exit-code LittleCoinCoin/main -- pnpm-lock.yaml` shows no new dependency [run]
- ⬜ `__reports__/updater_audit/01-macos_applescript_install_failure_v0.md` front-matter reads `status: resolved` [static]
**References**: `__reports__/updater_audit/01-macos_applescript_install_failure_v0.md` (failure strings, desired copy); `node_modules/@tauri-apps/plugin-updater/dist-js/index.d.ts` (`DownloadEvent` = `Started{contentLength?}` / `Progress{chunkLength}` / `Finished`; Tauri rejects with plain strings)

## Step 1: Pin the error classification with a failing test
**Goal**: Fix the copy contract before the implementation exists.
**Implementation Logic**:
Write `src/lib/stores/updater-errors.test.ts` (`node:test` + `node:assert`; Node 24 strips TS types natively, so no new dependency). It imports `classifyInstallError` from `./updater-errors.ts`. Cases and expected messages:
- permission denied, AppleScript cancel (`-128`, `User canceled`) and a read-only filesystem → `Couldn't install the update: move Sift to your Applications folder (or approve the administrator prompt) and try again.`
- signature failures → `The update failed verification and was not installed.`
- anything else → `Update failed: <raw>`
- non-string inputs are stringified

If `svelte-check` / `tsconfig` objects to the `.ts` extension import in the test, exclude `*.test.ts` from `tsconfig.json`'s `include` in this step and say so in the report.
**Deliverables**: `src/lib/stores/updater-errors.test.ts`
**Consistency Checks**: `node --test src/lib/stores/updater-errors.test.ts` (expected: FAIL)
**Commit**: `test(updater): pin install-error classification on report-01 strings`

## Step 2: Classifier implementation
**Goal**: A pure module the store and the E2E harness share.
**Implementation Logic**:
`src/lib/stores/updater-errors.ts` exports:
- `classifyInstallError(raw: unknown): string`, matching case-insensitively on substrings;
- `INSTALL_MESSAGES`, an object holding the exact copy strings, so the E2E leaves read the copy from source instead of duplicating it.

No imports from Tauri or Svelte.
**Deliverables**: `src/lib/stores/updater-errors.ts` (`classifyInstallError`, `INSTALL_MESSAGES`)
**Consistency Checks**: `node --test src/lib/stores/updater-errors.test.ts` (expected: PASS)
**Commit**: `fix(updater): classify install failures into actionable messages`

## Step 3: Truthful toast state machine
**Goal**: The user is never told an update is ready when it is not.
**Implementation Logic**:
Rewrite `src/lib/stores/updater.svelte.ts`, reusing `showToastReturningId`, `updateToastMessage`, `dismissToast` and `showToast` from `./log`:
1. **One updater toast id** (module-level `_toastId`). A check that finds an update while a toast is open replaces its message instead of stacking a new one.
2. **Install click:**
   - Ignore it if a download is in progress (keep the guard).
   - Otherwise dismiss the "available" toast, show `Downloading update…`, then `Downloading update… N%`, or `… N KB` without Content-Length.
3. **On `Finished`:** show `Installing update…`.
4. **After `await update.downloadAndInstall(...)` resolves:** show the persistent success toast `Update ready — restart to apply`, with the `Restart Now` action (relaunch errors stay surfaced).
5. **On any rejection:** dismiss the progress toast and show an error toast with `classifyInstallError(err)`. Remove the `isFinished` suppression entirely.
6. **Background check failure:** no toast. Log it through the existing log store, adding a minimal non-toast entry function to `src/lib/stores/log.ts` only if none exists, and keep `console.error`. A manual check keeps its warning toast.
7. **Copy strings** come from `INSTALL_MESSAGES` (extend it with the progress, available and ready strings).
**Deliverables**: `src/lib/stores/updater.svelte.ts` (`startDownload`, `checkForUpdates`, `initUpdaterStore`, `_toastId`); `src/lib/stores/updater-errors.ts` (`INSTALL_MESSAGES` extended); optionally `src/lib/stores/log.ts` (a non-toast log entry function)
**Consistency Checks**: `pnpm check && pnpm build && ! grep -q isFinished src/lib/stores/updater.svelte.ts` (expected: PASS)
**Commit**: `fix(updater): show restart only after the install succeeds`

## Step 4: Close report 01
**Goal**: The audit trail reflects the fix.
**Implementation Logic**:
In `__reports__/updater_audit/01-macos_applescript_install_failure_v0.md`, set the front-matter `status: resolved` and add a short `## Resolution` section naming `classifyInstallError` and the commit. Do not touch `__reports__/updater_audit/README.md`; docs_closure owns it. **Commit type:** use `report(updater-audit): …` as CONTRIBUTING defines. The roadmap grammar only admits the stock conventional types, so the Commit field below reads `docs`. The actual commit uses `report`.
**Deliverables**: `__reports__/updater_audit/01-macos_applescript_install_failure_v0.md` (`status: resolved`, `## Resolution`)
**Consistency Checks**: `grep -q '^status: resolved' __reports__/updater_audit/01-macos_applescript_install_failure_v0.md` (expected: PASS)
**Commit**: `docs(updater-audit): mark install-failure notice resolved`

## Gotchas
- **VERIFIER leaf.** Attack: is there any path where a failure still produces "ready" or a lingering progress toast? Can a double click start two downloads? Does a manual re-check during a download clobber the progress toast? Is a relaunch failure visible?
- `Toast.svelte` already renders `action` buttons; do not restyle it.
- The `.claude/launch.json` and E2E harness belong to e2e_ui; do not add them here.
