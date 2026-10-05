# Auto-Updater UX Harness — Results (v0)

Date: 2026-10-05

## Executive Summary

- **Result:** all 13 update-UX scenarios PASS against the real `updater.svelte.ts` and the real
  `Toast.svelte`, in headless Chrome, with machine-readable pass/fail (`bash scripts/e2e-ui.sh`
  exits 0; repeated runs agreed; a full run takes about 15 s).
- **The harness can fail:** run against the pre-campaign store (`bfef3c5`), the four install-failure
  scenarios FAIL with "ready shown after failure" (4/4 detected). That store shows the ready toast
  on `Finished` even when the install then fails.
- **What is real and what is faked:** only the Tauri modules the store imports are replaced
  (`@tauri-apps/plugin-updater`, `@tauri-apps/plugin-process`, `@tauri-apps/api/event`), through
  aliases that exist only in `vite.e2e.config.ts`. The fakes reject with plain strings as Tauri does
  and are type-checked against the real plugin signatures. The harness reads the rendered `.toast`
  DOM and clicks the rendered `.toast-action` / `.toast-dismiss` buttons. All copy is quoted
  through `INSTALL_MESSAGES`.
- **Limitations:**
  - `latest-older` is decided by the fake: the real "is the server version newer" comparison lives
    in the Rust plugin, so this scenario proves only how the store reacts to a `null` result.
  - Clicks are synthetic `.click()` calls on the rendered buttons, not trusted user events.
- **Not covered:** the real plugin's bytes-on-disk install, signature checking and relaunch (those
  belong to the native update E2E, report 05), and macOS-specific error text beyond the strings the
  fakes inject.

## Scenario results

Command: `bash scripts/e2e-ui.sh --verbose` (equivalently `pnpm e2e:ui`).

| Scenario | Result | Checks | What it proves |
|:---------|:-------|-------:|:---------------|
| success | PASS | 10 | percent progress, installing, then ready; a late `Finished` after settle is ignored; Restart Now calls `relaunch()` once |
| no-content-length | PASS | 6 | progress in KB (512, 1024), never a percentage |
| readonly-volume | PASS | 9 | permission copy; ready never shown; no progress toast left |
| permission-denied | PASS | 9 | permission copy; ready never shown; no progress toast left |
| admin-cancel | PASS | 9 | permission copy; ready never shown; no progress toast left |
| signature-fail | PASS | 9 | verification copy; ready never shown; no progress toast left |
| check-fail-background | PASS | 5 | no toast at all; one `warn` log entry with the background-failure prefix |
| check-fail-manual | PASS | 7 | stale Install toast replaced by a warn toast, auto-dismissed after about 8 s |
| up-to-date | PASS | 6 | silent on the background check; info toast on the manual check, auto-dismissed after about 4 s |
| latest-older | PASS | 6 | a server version older than the running one behaves as up to date |
| double-install-click | PASS | 6 | two Install clicks in one tick (before Svelte removes the button) cause one download |
| recheck-while-toast-open | PASS | 13 | re-checks with Install open, mid-download and after ready keep one updater toast; a dismissed progress or ready toast is re-shown |
| download-network-fail | PASS | 10 | connection drops before `Finished`: `failedPrefix` copy, no Installing toast, never ready, one toast |

Per the adversarial review's mutation test, removing both store guards makes the same-tick double click in `double-install-click` fail. (An earlier third click on the detached button was removed: detached nodes never reach Svelte's delegated handler, so it proved nothing.)

Every scenario also asserts that at most one toast is on screen at any observed state, that no
uncaught error occurred, and that at least one expectation ran (a run with zero checks fails).

## Toast sequences observed

Each line is a distinct DOM state of the toast stack, in order. `{…}` is the action button.

| Scenario | Sequence |
|:---------|:---------|
| success | `Update 9.9.9 available {Install}` → `Downloading update…` → `… 25%` → `… 50%` → `… 75%` → `… 100%` → `Installing update…` → `Update ready — restart to apply {Restart Now}` |
| no-content-length | available → `Downloading update…` → `… 512 KB` → `… 1024 KB` → `Installing update…` → ready |
| readonly-volume, permission-denied, admin-cancel | available → `Downloading update…` → `… 50%` → `… 100%` → `Installing update…` → error toast with the permission copy |
| signature-fail | same as above, ending in the error toast with the verification copy |
| check-fail-background | (no toast ever shown); log entry `warn: Background update check failed: …` |
| check-fail-manual | available → `warn: Couldn't check for updates: request timed out` → (empty) |
| up-to-date, latest-older | `info: Sift is up to date.` → (empty) |
| double-install-click | same as success, one download |
| download-network-fail | available → `Downloading update…` → `… 25%` → `error: Update failed: error sending request for url (https://github.com/…): connection closed before message completed` |
| recheck-while-toast-open | available → `Downloading update…` → (empty, dismissed) → `Downloading update…` (re-shown) → percentages → `Installing update…` → ready → (empty, dismissed) → ready (re-shown) |

## Negative control

Command: `bash scripts/e2e-ui.sh --negative-control` (exit 1, working tree restored; verified with
`git diff --exit-code src/lib/stores/updater.svelte.ts`). It swaps in
`git show bfef3c5:src/lib/stores/updater.svelte.ts`, builds into `dist-e2e/negative`, restores the
store, then runs all scenarios.

| Failure scenario | Against the pre-campaign store |
|:-----------------|:-------------------------------|
| readonly-volume | FAIL: ready shown after failure |
| permission-denied | FAIL: ready shown after failure |
| admin-cancel | FAIL: ready shown after failure |
| signature-fail | FAIL: ready shown after failure |

The other scenarios give a wider picture of what the campaign changed. Against the old store,
11 of 13 scenarios fail in total; `up-to-date` and `latest-older` pass (`download-network-fail` fails there because the raw error is shown without the `failedPrefix`). Its remaining defects:
the Install toast stays open beside the progress toast (two toasts at once), a manual re-check
adds more Install toasts, a manual-check failure shows the raw error without the prefix, and a
background failure leaves no log entry.

Exit codes of the script: 0 all PASS; 1 a scenario failed (in negative-control mode: the failure
scenarios failed as intended); 2 environment or harness error (including Chrome yielding no DOM and runner and harness
scenario lists drifting apart); 3 the negative control did not detect the old defect.

## Commands

```
pnpm install --frozen-lockfile
bash scripts/e2e-ui.sh                       # all scenarios, exit 0 only if all PASS
bash scripts/e2e-ui.sh --verbose             # plus each scenario's toast sequence
bash scripts/e2e-ui.sh --only up-to-date     # a subset
bash scripts/e2e-ui.sh --negative-control    # exits 1 when the old defect is detected
pnpm exec vite build --config vite.e2e.config.ts --outDir dist-e2e
pnpm build && ! grep -rqE '__e2e|lib/e2e' dist
git diff --exit-code bfef3c5 -- pnpm-lock.yaml
```

Visual pass: `pnpm exec vite --config vite.e2e.config.ts --port 4174` (launch entry `sift-e2e-ui`),
then navigate the Browser pane to `http://localhost:4174/e2e.html` (the scenario list) or
`http://localhost:4174/e2e.html?scenario=<name>`. `launch.json` cannot carry a path and `/` serves the
production `index.html`, so the navigation step is required. Dev mode runs in real time (the background check fires after 4 s).

## Measured notes

- `--virtual-time-budget=20000` fast-forwards the production timers (4 s background check, 4 s and
  8 s auto-dismiss) with no change to production delays. `performance.now()` follows virtual time,
  so the auto-dismiss durations are asserted (3.5 to 5.5 s and 7.5 to 9.5 s).
- With `--headless=new --dump-dom`, this Chrome prints the DOM and then does not exit. The runner
  writes the DOM to a file, treats the closing `</html>` as completion and kills the browser.
- An interrupted run (SIGINT or SIGTERM) exits 130 or 143, kills the preview server and any Chrome
  started with that run's own profile directory, and removes its temp files.
- The harness page runs one scenario per load, because `initUpdaterStore()` is once-only.
