# E2E Keychain Assertions

**Goal**: The native E2E proves that the API key really persists, records what happens to it after an update, and proves the IPC stays responsive while the Keychain read is pending, instead of only printing probe lines.
**Pre-conditions**:
- [x] Working on `task/e2e_keychain_assertions`, branched from the integration tip `milestone/keychain-persistence`
- [x] `pnpm install --frozen-lockfile` done in the leaf worktree
**Success Gates**:
- ✅ `bash -n scripts/e2e-updater.sh` exits 0 [run]
- ✅ `pnpm check && pnpm build` pass [run]
- ✅ `grep -c 'KEYCHAIN_SERVICE}api-key' scripts/e2e-updater.sh` prints `0`, and `grep -c 'in-memory mock' scripts/e2e-updater.sh` prints `0` [static]
- ✅ `grep -q 'KEYCHAIN_AFTER_UPDATE=' scripts/e2e-updater.sh && grep -q 'ipc alive' src/lib/e2e/updater-driver.ts` exits 0 [static]
- ✅ `git diff --exit-code df80002 -- pnpm-lock.yaml src-tauri` exits 0 (this leaf touches neither the lockfile nor the backend; the base is the leaf's branch point because the integration tip moves as siblings merge) [run]
- ✅ At L1 close, the maintainer runs `pnpm e2e:native` on the integration tip. It exits 0, and the transcript has `PASS  keychain item 'sift-e2e' written by 0.0.1`, `KEYCHAIN_AFTER_UPDATE=blocked` (expected under ad-hoc signing) and `PASS  ipc stays responsive while the keychain read is pending` [behavioral]
**References**: `__reports__/api_key_storage/01-keychain_signing_measurement_v0.md` (defect 1, run 1b/2b timelines); `scripts/e2e-updater.sh` (`verify_keychain_isolation`, `run_happy_path`, `keychain_presence`, `wait_for_log_after`, `log_has`); `src/lib/e2e/updater-driver.ts` (`probeKeychain`, `post`); `src-tauri/src/settings.rs` (`get_settings`, a sync command on the main thread); experiment commit `272b88b` on `experiment/keychain-signing` (a working version of Step 1's isolation change)

## Step 1: Isolation guard that survives the apple-native layout
**Goal**: The guard still refuses to launch a build that could use the real `sift` item, without depending on `.rodata` byte order.
**Implementation Logic**:
In `verify_keychain_isolation`, build B's check needs `${KEYCHAIN_SERVICE}api-key` to be adjacent bytes. With `apple-native` the linker puts `sift-e2e` after the USER constants, so the check rejects a correct build (report 01, defect 1).
Replace it with the literal-presence check build A already uses (`LC_ALL=C grep -aq "$KEYCHAIN_SERVICE" "$bin_b"`), with die/pass messages matching A's. Keep both source-level guards unchanged: `SIFT_KEYRING_SERVICE` is read in exactly one file, and `sift-e2e` appears in no other Rust source.
Rewrite the comment above the function:
- B now rests on the same argument as A, the literal plus the single-consumer check.
- Drop the line "Not proven here: that the keyring backend is real. It is the in-memory mock". Step 2's roundtrip and presence assertions now prove the backend is real.

This change is correct on today's mock build too, so the leaf has no ordering dependency on `apple_native_backend`.
**Deliverables**: `scripts/e2e-updater.sh` (`verify_keychain_isolation` B check and its comment)
**Consistency Checks**: `bash -n scripts/e2e-updater.sh && ! grep -q 'KEYCHAIN_SERVICE}api-key' scripts/e2e-updater.sh && ! grep -q 'in-memory mock' scripts/e2e-updater.sh` (expected: PASS)
**Commit**: `test(e2e): check the keyring service literal without relying on rodata order`

## Step 2: Assert persistence, record the post-update outcome, prove IPC stays live
**Goal**: The E2E fails if the key is not persisted, prints one machine-readable outcome for the post-update read, and catches a main-thread freeze.
**Implementation Logic**:
1. **Driver (`src/lib/e2e/updater-driver.ts`, `probeKeychain`), branch at or above `TARGET_VERSION`:**
   - Start `invoke<string>('get_api_key')` without awaiting it.
   - After 1000 ms, time `await invoke('get_settings')` with `performance.now()`. A rejection counts as an answer and is timed the same way. Post `ipc alive` if it answered within `IPC_ALIVE_MAX_MS = 2000`, otherwise post `ipc slow|<ms>`.
   - Then await the pending read and post `keychain ok` or `keychain error|...` as today.
   - Why: `get_settings` is a sync command, so it runs on the main thread. If the pending Keychain read held the main thread, `get_settings` would answer only when the dialog is answered, so it would be slow or never answer.
   - Timing it in the driver is what makes the check discriminate. An untimed `ipc alive` would still arrive on time whenever a human answered the dialog within the script's wait (amended after verification, 2026-10-06).
   - The branch below `TARGET_VERSION` is unchanged.
2. **Script, preflight (`main`) and after `booted $OLD_VERSION` (`run_happy_path`):**
   - Preflight, right after the real-keychain baseline: remove any stale `$KEYCHAIN_SERVICE` item left by an aborted run (the same bounded delete loop as the cleanup), and `die` if one remains. Otherwise a stale item would make the presence assertion vacuous, and it could also prompt on B's write.
   - Wait (bounded, about 10 s) for `keychain roundtrip ok`, and `fail` if it is missing or `roundtrip mismatch` appears.
   - Then `keychain_presence "$KEYCHAIN_SERVICE"` must print `present`: `pass "keychain item '$KEYCHAIN_SERVICE' written by $OLD_VERSION"`, otherwise `fail`. This is an attributes-only lookup, so it never prompts.
3. **Script, after `booted $NEW_VERSION`.** Only if 0.2.0 booted (`booted_new=1`; otherwise its `fail` is already counted and no outcome is printed): keep the existing 20 s `wait_for_log_after ... "keychain (ok|error)"`, then:
   - Emit exactly one line, `KEYCHAIN_AFTER_UPDATE=ok`, `KEYCHAIN_AFTER_UPDATE=blocked` (no result) or `KEYCHAIN_AFTER_UPDATE=error`. For `blocked`, add a `say` noting that a login-keychain dialog is expected under ad-hoc signing.
   - `fail` on `error`, because the stored key was lost or unreadable.
   - `wait_for_log_after "booted $NEW_VERSION\$" "ipc (alive|slow)"` with a bound of about 10 s.
   - Then `pass "ipc stays responsive while the keychain read is pending"` on `ipc alive`. When the outcome was `ok`, the message notes that the check did not exercise a freeze.
   - `fail` on `ipc slow|<ms>` or on no line.
   - Replace the informational `say "keychain probe: ..."` line with these assertions.
4. Change nothing in the read-only cases, and add no environment knobs.
**Deliverables**: `src/lib/e2e/updater-driver.ts` (`probeKeychain` posting `ipc alive`); `scripts/e2e-updater.sh` (`run_happy_path` assertions, `KEYCHAIN_AFTER_UPDATE` line)
**Consistency Checks**: `bash -n scripts/e2e-updater.sh && grep -q 'KEYCHAIN_AFTER_UPDATE=' scripts/e2e-updater.sh && grep -q 'ipc alive' src/lib/e2e/updater-driver.ts && pnpm check && pnpm build` (expected: PASS)
**Commit**: `test(e2e): assert keychain persistence and responsive IPC across an update`
