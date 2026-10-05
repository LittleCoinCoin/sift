# E2E Native Update

**Goal**: Prove with real signed binaries that an installed old Sift detects, downloads, verifies, installs and restarts into the new version. The real buttons are clicked by an in-app driver, and the user's real Sift data and keychain are never touched.
**Pre-conditions**:
- [ ] Working on `task/e2e_native`, branched from the integration tip after L1
- [ ] Port 1430 free; `/Applications/Google Chrome.app` not required
**Success Gates**:
- ⬜ `bash scripts/e2e-updater.sh` exits 0. The `/e2e-log` dump shows, in order: `available` → `downloading N%` (at least one 0<N<100) → `installing` → `ready` → `booted 0.2.0` [run]
- ⬜ After the run, `defaults read "$TMPDIR/sift-e2e/Sift.app/Contents/Info" CFBundleShortVersionString` prints `0.2.0`, and the script reports a different PID before and after the restart [run]
- ⬜ The read-only-volume case, run by the same script from an `hdiutil` image, logs the `INSTALL_MESSAGES` readonly copy and never `ready` [run]
- ⬜ `pnpm build && ! grep -rq 'e2e-log' dist` — the driver is absent from a normal build [run]
- ⬜ `security find-generic-password -s sift >/dev/null 2>&1` gives the same result before and after the run (the real keychain item is untouched), and no `sift-e2e` item remains [run]
**References**: `__reports__/updater_audit/00-full_install_testing_constraints_v0.md` (plain-http endpoints are only accepted under `debug_assertions`); `__reports__/updater_audit/03-full_e2e_test_definition_v0.md` (original scenarios, now retargeted to 0.2.0); `src-tauri/src/keyring_store.rs` (`SERVICE = "sift"` hardcoded)

## Step 1: Isolated keyring service for e2e builds
**Goal**: An e2e build can never read or write the user's real Sift keychain items.
**Implementation Logic**:
In `src-tauri/src/keyring_store.rs`, replace the constant with `const SERVICE: &str = match option_env!("SIFT_KEYRING_SERVICE") { Some(s) => s, None => "sift" };`. This is a compile-time override, and production is unchanged. Add a unit test asserting `SERVICE == "sift"` when the variable is unset. Builds made by the e2e script set `SIFT_KEYRING_SERVICE=sift-e2e`.
**Deliverables**: `src-tauri/src/keyring_store.rs` (`SERVICE`, test `service_defaults_to_sift`)
**Consistency Checks**: `cd src-tauri && cargo test --lib service_defaults_to_sift` (expected: PASS)
**Commit**: `test(e2e): isolate the keyring service of e2e builds`

## Step 2: In-app driver behind a build-time flag
**Goal**: The real Install and Restart Now buttons are clicked inside the real native webview.
**Implementation Logic**:
1. `src/main.ts`: add `if (import.meta.env.VITE_SIFT_E2E === '1') void import('./lib/e2e/updater-driver');`. Measured: Vite drops the chunk when the flag is unset.
2. `src/lib/e2e/updater-driver.ts`:
   - Posts `booted <version>` (from `@tauri-apps/api/app` `getVersion`).
   - Observes `.toast` text and posts each change as `text/plain` (a simple request, so no CORS preflight) to `http://127.0.0.1:1430/e2e-log`.
   - Clicks `.toast-action` when the text matches the `INSTALL_MESSAGES` available copy, and again for the ready copy.
   - Keychain probe: on first boot below 0.2.0 it calls `invoke('set_api_key', {key: 'e2e-probe'})`. On boot at 0.2.0 it calls `get_api_key` and posts `keychain ok|<error>`.
**Deliverables**: `src/main.ts` (gated import); `src/lib/e2e/updater-driver.ts` (`startDriver`, `post`)
**Consistency Checks**: `pnpm build && ! grep -rq 'e2e-log' dist` (expected: PASS)
**Commit**: `test(e2e): add in-app updater driver gated behind VITE_SIFT_E2E`

## Step 3: Mock update server
**Goal**: Serve a real signed artifact and collect the driver's log.
**Implementation Logic**:
Rework `scripts/serve-mock-update.py` (stdlib only, run with `uv run`). It takes `--artifact <A.app.tar.gz>` `--version <V>` `--port 1430`.
- `GET /latest.json`: `darwin-aarch64` and `darwin-x86_64` entries, whose `signature` is the `.sig` file content verbatim and whose `url` points at `/artifact.tar.gz`.
- `GET /artifact.tar.gz`: streams with `Content-Length`.
- `POST /e2e-log`: appends a timestamped line.
- `GET /e2e-log`: dumps all lines.
- Every request is logged to stderr with a timestamp.
- Without `--artifact`, the old notification-only behaviour stays available.
**Deliverables**: `scripts/serve-mock-update.py` (`Handler.do_GET`, `Handler.do_POST`, args `--artifact`, `--version`, `--port`)
**Consistency Checks**: `uv run python -m py_compile scripts/serve-mock-update.py` (expected: PASS)
**Commit**: `test(e2e): serve a signed artifact and collect driver logs in the mock server`

## Step 4: Orchestrator
**Goal**: One command builds both apps, runs the happy path and the read-only case, asserts, and cleans up.
**Implementation Logic**:
`scripts/e2e-updater.sh`. Everything lives under `W=$TMPDIR/sift-e2e`.
1. **Keys and env:** generate a throwaway key (`pnpm tauri signer generate --ci -p '' -w $W/key`), then export `TAURI_SIGNING_PRIVATE_KEY`, `_PASSWORD=''`, `SIFT_KEYRING_SERVICE=sift-e2e` and `VITE_SIFT_E2E=1`.
2. **Build A:**
   - `pnpm tauri build --bundles app --config <json>`, with identifier `dev.eliottjacopin.sift.e2e`, version `0.2.0`, endpoint `http://127.0.0.1:1430/latest.json`, the throwaway pubkey, and CSP `connect-src 'self' ipc: http://ipc.localhost http://127.0.0.1:1430`.
   - Copy out the `.app.tar.gz` and `.sig`.
3. **Build B:** `pnpm tauri build --debug --bundles app` with the same overrides but version `0.0.1`. Copy `Sift.app` to `$W/Sift.app`.
4. **Happy path:**
   - Start the mock server; snapshot `security find-generic-password -s sift` presence; launch `$W/Sift.app/Contents/MacOS/*`.
   - Poll `GET /e2e-log` with a timeout (no bare sleep loops without a bound), and `screencapture -x` at each stage into `$W/shots/`.
   - Assert the sequence, the PID change (`pgrep -f "$W/Sift.app"`), and `CFBundleShortVersionString`.
5. **Read-only case:** reinstall B into a `hdiutil create -format UDRO` image, attach it, launch from `/Volumes/...`, and assert the readonly copy and no `ready`.
6. **Cleanup in a trap:** kill the apps and server, detach the image, `security delete-generic-password -s sift-e2e`, and re-assert the real `sift` item presence is unchanged.

Add `"e2e:native": "bash scripts/e2e-updater.sh"` to `package.json`.
**Deliverables**: `scripts/e2e-updater.sh` (functions `build_a`, `build_b`, `run_happy_path`, `run_readonly_case`, `cleanup`); `package.json` (`e2e:native` script)
**Consistency Checks**: `bash -n scripts/e2e-updater.sh` (expected: PASS)
**Commit**: `test(e2e): orchestrate native download-install-restart runs`

## Step 5: Run and report
**Goal**: Durable evidence, including the keychain behaviour across the ad-hoc identity change.
**Implementation Logic**:
1. Run `bash scripts/e2e-updater.sh`.
2. Write `__reports__/updater_audit/05-e2e_native_results_v0.md`:
   - the log sequence, PIDs and version;
   - the read-only result;
   - the keychain probe result (`keychain ok` or the prompt/error, with the screenshot if a system dialog appeared);
   - timings;
   - the exact commands.
3. Copy 3–5 key screenshots, downscaled with `sips -Z 1200`, to `__reports__/updater_audit/05-assets/`.
4. Do not touch the reports README (docs_closure). **Commit type:** `report(updater-audit): …`. The roadmap grammar forces `docs` in the field below.
**Deliverables**: `__reports__/updater_audit/05-e2e_native_results_v0.md`; `__reports__/updater_audit/05-assets/*.png`
**Consistency Checks**: `test -s __reports__/updater_audit/05-e2e_native_results_v0.md` (expected: PASS)
**Commit**: `docs(updater-audit): record native update E2E results`

## Gotchas
- **VERIFIER leaf.** Attack:
  - Does the driver click the real buttons, or could the sequence be faked by the server?
  - Can the "never ready" assertion actually fail?
  - Can the driver reach a normal `dist/`?
  - Is the real `sift` keychain item ever read or written?
  - Does cleanup run on failure?
- **Never** install into or modify `/Applications`. The admin-prompt cancel case stays with the user (publish_cycle).
- Build A and Build B both carry the driver (`VITE_SIFT_E2E=1`): A needs it to report `booted 0.2.0` and the keychain probe. A sees `latest.json` at 0.2.0 equal to itself, so it does not loop.
- Both builds need the throwaway signing key, because `createUpdaterArtifacts` is on. Never use or request the production key.
- **[nice]** An x86_64 leg under Rosetta (`--target x86_64-apple-darwin`, `arch -x86_64`) if time allows. Record it as not run otherwise.
