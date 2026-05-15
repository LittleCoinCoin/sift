# Implement Updater Store and Wire into App

**Goal**: Create `src/lib/stores/updater.svelte.ts` with the full update-check, download, and restart flow, then wire it into `App.svelte`'s `onMount`.
**Pre-conditions**:
- [ ] `deps.md` done — `@tauri-apps/plugin-updater` and `@tauri-apps/plugin-process` installed
- [ ] `toast_extension.md` done — `action?`, `persistent?`, `showToastReturningId`, and `updateToastMessage` available from `src/lib/stores/log.ts`
- [ ] Branch created from `main`
**Success Gates**:
- ✅ `pnpm check` passes with no TypeScript errors [run]
- ✅ Local mock test: launching with a lower `version` in `tauri.conf.json` and `endpoint` pointing to a local `latest.json` shows an "Update available" toast with an **Install** button ~4 s after launch [behavioral]
- ✅ Clicking **Install** starts the download and the toast message updates with progress [behavioral]
- ✅ `initUpdaterStore` is called in `App.svelte`'s `onMount` alongside `initLogStore` and `initJobStore` [static]
**References**: [R01 Plan](../../../../.claude/plans/assuming-the-current-project-enchanted-meteor.md) — store design and testing strategy (§ Work breakdown E, F, and Testing)

---

## Step 1: Scaffold the module and event listener

**Goal**: Create the `updater.svelte.ts` module with the `initUpdaterStore` export, the startup auto-check delay, and the `check-for-updates` event listener.

**Implementation Logic**: Create `src/lib/stores/updater.svelte.ts`. Follow the same init-guard pattern as `initLogStore` in `log.ts` (a module-level `_initialized` boolean). `initUpdaterStore` should: (1) guard against double-init, (2) register a `listen('check-for-updates', ...)` listener that calls `checkForUpdates(true)` (explicit = manual trigger), (3) schedule `checkForUpdates(false)` via `setTimeout` with a ~4 s delay so the startup check fires after the initial render settles. Export only `initUpdaterStore` from this module.

**Deliverables**: `src/lib/stores/updater.svelte.ts` — module with `initUpdaterStore` export, `_initialized` guard, `listen` call, `setTimeout` startup check

**Consistency Checks**: `pnpm check` (expected: PASS)

**Commit**: `feat(updater): scaffold updater store with init and event listener`

---

## Step 2: Implement checkForUpdates

**Goal**: Call the plugin's `check()` function and react with appropriate toasts.

**Implementation Logic**: Add `checkForUpdates(explicit: boolean)` as a module-private async function. Import `check` from `@tauri-apps/plugin-updater`. Call `check()` inside a try/catch. On success: if the returned `Update` object is non-null, show a persistent toast (via `showToastReturningId`) at level `'info'` with message `"Update <version> available"` and an `action` of `{ label: 'Install', onClick: () => startDownload(update) }`. If the result is null and `explicit` is true, show a brief non-persistent `'info'` toast saying "Sift is up to date." (no action, auto-dismisses normally). On catch: if `explicit` is true, show a `'warn'` toast saying "Could not reach update server."; if `explicit` is false (auto-check), swallow the error silently — a background check failure should not interrupt the user.

**Deliverables**: `src/lib/stores/updater.svelte.ts` — `checkForUpdates` function with `check()` call, toast branching logic, and error handling

**Consistency Checks**: `pnpm check` (expected: PASS)

**Commit**: `feat(updater): implement update check with toast feedback`

---

## Step 3: Implement startDownload with progress and restart

**Goal**: Download and install the update with live progress feedback, then offer a restart.

**Implementation Logic**: Add `startDownload(update: Update)` as a module-private async function. Import `relaunch` from `@tauri-apps/plugin-process`. Call `showToastReturningId` to create a persistent `'info'` toast saying "Downloading update…" and capture its `id`. Call `update.downloadAndInstall(onEvent)` where `onEvent` handles three event types: on `'Started'`, capture `contentLength` for percentage math; on `'Progress'`, accumulate `chunkLength` and call `updateToastMessage(id, "Downloading update… XX%")` with the running percentage; on `'Finished'`, dismiss the progress toast via `dismissToast(id)` and show a new persistent `'success'` toast with message "Update ready — restart to apply" and action `{ label: 'Restart Now', onClick: () => relaunch() }`. Wrap the entire `downloadAndInstall` call in a try/catch: on error, dismiss the progress toast and show an `'error'` toast with the error message.

**Deliverables**: `src/lib/stores/updater.svelte.ts` — `startDownload` function with `downloadAndInstall` call, `onEvent` progress handler, restart toast, error handling

**Consistency Checks**: `pnpm check` (expected: PASS)

**Commit**: `feat(updater): implement download with progress and restart toast`

---

## Step 4: Wire initUpdaterStore into App.svelte

**Goal**: Activate the updater store at app startup alongside the existing stores.

**Implementation Logic**: In `src/App.svelte`, add an import for `initUpdaterStore` from `./lib/stores/updater.svelte`. In the existing `onMount` callback (currently `onMount(() => { initLogStore(); initJobStore(); })`), append `initUpdaterStore()` as a third call. No other changes to `App.svelte` are needed — the store is self-contained and mounts its own listeners.

**Deliverables**: `src/App.svelte` — import of `initUpdaterStore`; call to `initUpdaterStore()` inside `onMount`

**Consistency Checks**: `pnpm check` (expected: PASS)

**Commit**: `feat(updater): wire initUpdaterStore into App onMount`
