# E2E UX Harness

**Goal**: Exercise every update-UX state against the real store and the real `Toast.svelte` in a browser, with machine-readable pass/fail, so the "click to download, click to install" UX is proven without a human.
**Pre-conditions**:
- [ ] Working on `task/e2e_ui`, branched from the integration tip after L1 (install_feedback merged)
- [ ] `/Applications/Google Chrome.app` present (measured) for headless runs
**Success Gates**:
- ⬜ `bash scripts/e2e-ui.sh` exits 0, and its summary lists all 12 scenarios as `PASS`: success, no-content-length, readonly-volume, permission-denied, admin-cancel, signature-fail, check-fail-background, check-fail-manual, up-to-date, latest-older, double-install-click, recheck-while-toast-open [run]
- ⬜ `bash scripts/e2e-ui.sh --negative-control` exits non-zero, with the failure scenarios reporting "ready shown after failure" against the pre-campaign store (`git show bfef3c5:src/lib/stores/updater.svelte.ts`). The working tree is restored afterwards (`git diff --exit-code src/lib/stores/updater.svelte.ts`) [run]
- ⬜ `pnpm build && ! grep -rqE '__e2e|lib/e2e' dist` — the harness never reaches the production bundle [run]
- ⬜ `git diff --exit-code LittleCoinCoin/main -- pnpm-lock.yaml` — no new dependency [run]
**References**: `node_modules/@tauri-apps/plugin-updater/dist-js/index.d.ts` (`check()` returns `Update | null`; `DownloadEvent` union); `src/lib/stores/updater-errors.ts` (`INSTALL_MESSAGES` copy, owned by install_feedback); `src/lib/Toast.svelte` (`.toast-action` buttons)

## Step 1: Scenario fakes and self-driving harness
**Goal**: The real store and component run in a browser against scripted plugin behaviour.
**Implementation Logic**:
1. `vite.e2e.config.ts` extends `vite.config.ts`. Its `resolve.alias` maps the following to files in `src/lib/e2e/fakes/`:
   - `@tauri-apps/plugin-updater` → `updater.ts`: `check()` returns a fake `Update` whose `downloadAndInstall(cb)` emits `Started`/`Progress`/`Finished` per scenario, then resolves or **throws a plain string**, as Tauri does;
   - `@tauri-apps/plugin-process` → `process.ts`: `relaunch()` records a call;
   - `@tauri-apps/api/event` → `event.ts`: `listen()` captures handlers so the harness can fire `check-for-updates`;
   - plus `@tauri-apps/api/core` only if the harness imports components that `invoke`.
2. `e2e.html` + `src/lib/e2e/harness.ts`:
   - Mounts `Toast.svelte` and calls the real `initUpdaterStore()`.
   - Reads `?scenario=`, observes `.toast` text changes (MutationObserver), and clicks the real `.toast-action` buttons as a user would.
   - Evaluates the scenario's expectations, quoting copy only through `INSTALL_MESSAGES`.
   - Publishes `window.__e2e = {scenario, pass, failures, toasts}` and writes it as JSON into `<pre id="e2e-result">`.
3. Scenario expectations:
   - Every failure scenario must never show the ready copy, and must leave no progress toast.
   - `double-install-click` produces exactly one download.
   - `recheck-while-toast-open` keeps a single updater toast.
   - `latest-older` and `up-to-date` show "up to date" only on a manual check.
   - `check-fail-background` shows no toast.
**Deliverables**: `vite.e2e.config.ts`; `e2e.html`; `src/lib/e2e/harness.ts` (`SCENARIOS`, `runScenario`); `src/lib/e2e/fakes/updater.ts` (`check`); `src/lib/e2e/fakes/process.ts` (`relaunch`); `src/lib/e2e/fakes/event.ts` (`listen`, `emitForTest`)
**Consistency Checks**: `pnpm exec vite build --config vite.e2e.config.ts --outDir dist-e2e` (expected: PASS)
**Commit**: `test(e2e): add self-driving browser harness for updater UX scenarios`

## Step 2: Runner script and launch entry
**Goal**: One command runs all scenarios headless; the Browser pane can open the same harness for visual inspection.
**Implementation Logic**:
1. `scripts/e2e-ui.sh`:
   - Builds with `vite.e2e.config.ts` into `dist-e2e/` (add `dist-e2e/` to `.gitignore`).
   - Serves it with `pnpm exec vite preview --config vite.e2e.config.ts --port 4174 --strictPort` in the background, with a trap to kill it.
   - For each scenario, runs `"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new --disable-gpu --virtual-time-budget=20000 --dump-dom "http://127.0.0.1:4174/e2e.html?scenario=<s>"`.
   - Extracts `#e2e-result` JSON with node, prints a PASS/FAIL table, and exits non-zero on any FAIL.
2. `--negative-control`:
   - Copies the current store to `$TMPDIR` and writes `git show bfef3c5:src/lib/stores/updater.svelte.ts` in its place.
   - Runs, then restores in a `trap` even on failure.
   - Inverts the exit status for the failure scenarios: it succeeds only if they FAIL.
3. Add `"e2e:ui": "bash scripts/e2e-ui.sh"` to `package.json`.
4. Create `.claude/launch.json` (it does not exist yet) with an entry `sift-e2e-ui` running `pnpm exec vite --config vite.e2e.config.ts --port 4174`.
**Deliverables**: `scripts/e2e-ui.sh` (flag `--negative-control`); `package.json` (`e2e:ui` script); `.claude/launch.json` (`sift-e2e-ui`); `.gitignore` (`dist-e2e/`)
**Consistency Checks**: `bash -n scripts/e2e-ui.sh` (expected: PASS)
**Commit**: `test(e2e): run updater UX scenarios headless with a negative control`

## Step 3: Run and report
**Goal**: Durable evidence of the UX for stakeholders.
**Implementation Logic**:
1. Run `bash scripts/e2e-ui.sh` and `--negative-control`, and capture both tables.
2. Write `__reports__/updater_audit/04-e2e_ui_results_v0.md` (findings report): the scenario table, each scenario's toast sequence, the negative-control result, and the exact commands.
3. Do not touch `__reports__/updater_audit/README.md` (docs_closure). **Commit type:** `report(updater-audit): …`. The roadmap grammar forces `docs` in the field below.
**Deliverables**: `__reports__/updater_audit/04-e2e_ui_results_v0.md`
**Consistency Checks**: `test -s __reports__/updater_audit/04-e2e_ui_results_v0.md` (expected: PASS)
**Commit**: `docs(updater-audit): record updater UX harness results`

## Gotchas
- **VERIFIER leaf.** Attack:
  - Does the harness click the real `.toast-action` buttons rather than call store functions?
  - Can a scenario pass vacuously (e.g. no toast at all)?
  - Do the fakes diverge from the real plugin types?
  - Does any harness code leak into `dist/`?
- The coordinator also opens the harness in the Browser pane (`preview_start` `sift-e2e-ui`) for a visual pass. Subagents may lack the pane tools, so the gate is the headless run.
- `initUpdaterStore` waits 4 s before the background check. `--virtual-time-budget` fast-forwards that; do not change the production delay for the test.
