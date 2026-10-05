# Native Update E2E Results (v0)

Date: 2026-10-05

---
type: findings
topic: updater_audit
produced-by: task/e2e_native (`pnpm e2e:native`, `scripts/e2e-updater.sh`)
date: 2026-10-05
domain: code
confidence: confirmed (full-rebuild run, exit 0, 20 PASS and 0 FAIL; an earlier run exposed the defects below)
---

## Headline

An installed Sift 0.0.1 (debug build, ad-hoc signed) detected 0.2.0 on a mock server, downloaded the
signed `.app.tar.gz` (5.57 MB, Content-Length present), verified it, replaced its own bundle,
and relaunched as the 0.2.0 release build. The Install and Restart Now buttons were clicked inside the real
webview by the in-app driver. The real `sift` keychain item and `/Applications` were never touched.

Findings that matter more than the pass:

| # | Finding | Evidence | Consequence |
|--:|---------|----------|-------------|
| 1 | `keyring = "3"` has no platform feature, so it uses the in-memory mock store. The app never writes a keychain item. | `cargo tree -p keyring -e features` lists only `log`. `set_api_key` then `get_api_key` in the same process returns `No matching entry found in secure storage`. The `security` tool never sees a `sift-e2e` item. | The API key is not persisted across launches. The keychain-across-identity-change question is moot until `apple-native` is enabled. Tracked as a separate task. The isolation check below proves where the service name comes from, not that the backend is real. |
| 2 | Running from a mounted disk image failed with raw `Update failed: Cross-device link (os error 18)`. **Fixed** in this branch (`fix(updater): explain cross-device install failures from a mounted dmg`). | First run: `std::fs::rename` from `$TMPDIR` to the image gives EXDEV, which `classifyInstallError` did not match. After the fix the realistic disk-image case shows the exact `INSTALL_MESSAGES.permission` copy. | The pattern `cross-device link` has a test case with the real string, and blanking it fails exactly that test. |
| 3 | Exec'ing `Contents/MacOS/sift` from a script does not boot the webview. | 9 of 9 standalone trials: process alive with 0% CPU and no JS, or abort `Failed to setup app: error encountered during setup hook: unknown path`. `open -n <bundle>` booted in about 3 s every time. | The script launches with `open -n`. Cause not isolated (location and the `com.apple.provenance` xattr were tried; the xattr cannot be cleared); it affects tests, not users, who launch from Finder. |
| 4 | Release builds accept `http://` endpoints with `plugins.updater.dangerousInsecureTransportProtocol: true`. | Build A sets it; its post-update check hit the mock server (3rd `GET /latest.json`). | The spec's "only debug builds" is not absolute. It lets the relaunched release app prove it does not loop. |

## Happy path (0.0.1 to 0.2.0), run of 13:41

Ordered `/e2e-log` (server timestamps; the 86 `Downloading update… N%` toasts from 0% to 100% are collapsed):

```
13:41:45.966 booted 0.0.1
13:41:45.971 keychain set ok
13:41:45.976 keychain error|No matching entry found in secure storage     (same-process get, see finding 1)
13:41:50.018 toast Update 0.2.0 available
13:41:52.159 click available (Install)
13:41:52.175 toast Downloading update…
13:41:52.183 toast Downloading update… 0%      ... 86 toasts ...   99%, 100%
13:41:53.962 toast Installing update…
13:41:54.228 toast Update ready — restart to apply
13:41:56.436 click ready (Restart Now)
13:41:57.166 booted 0.2.0
13:41:57.174 keychain error|No matching entry found in secure storage     (get at 0.2.0)
```

| Check | Result |
|-------|--------|
| Order available < click < N% (0<N<100) < installing < ready < click < booted 0.2.0 | pass (log lines 4, 5, 8, 93, 94, 95, 96); click patterns are anchored (`click ready \(`) so a "skipped" line cannot satisfy it |
| `defaults read $TMPDIR/sift-e2e/Sift.app/Contents/Info CFBundleShortVersionString` | `0.2.0` |
| Installed executable sha256 equals build A's and differs from build B's | pass |
| PID before restart / after | 27042 / 27236; the old process is gone (`logs/pids.txt`) |
| Update loop after restart | none; build A re-checked (3rd `GET /latest.json`) and showed no toast |
| Server served `GET /artifact.tar.gz` | yes |
| Timings | boot 2 s; available toast 4 s after boot; download 1.8 s (15 ms pause per 64 KiB); install 0.3 s; relaunch 0.7 s after the click |

## Disk-image cases (`hdiutil create -format UDRO`, mounted at `/Volumes/SiftE2E`)

Both cases must show the download at 100%, then `Installing update…`, then the exact permission copy, with no ready toast and no unclassified `Update failed:` toast. Order is asserted by log line (100% at 92, installing at 93, permission copy at 94).

```
13:42:25.983 toast Downloading update… 100%
13:42:26.006 toast Installing update…
13:42:26.258 toast Couldn't install the update: move Sift to your Applications folder (or approve the administrator prompt) and try again.
```

| Case | Launch | Error | Bundle / process |
|------|--------|-------|------------------|
| `image-realistic` | default env, as a user running from the DMG | EXDEV, now shown as the permission copy | still 0.0.1, PID 27461 unchanged |
| `tmpdir-on-volume` (secondary) | `TMPDIR` on the volume, so EROFS | the permission copy | still 0.0.1, PID 27698 unchanged |

The "never ready" check is meaningful: the same pattern matches in the happy-path log (control assertion). Detaching the image is bounded (30 s, then `-force` once); a failure keeps the device recorded and is reported as a FAIL.

## Keychain

- Probe result: `keychain set ok`, then `keychain error|No matching entry found in secure storage` at both versions (finding 1). No system dialog appeared.
- `security find-generic-password -s sift`: absent before and after (script assertion). No `sift-e2e` item remained.
- Isolation proof: `SIFT_KEYRING_SERVICE` is read only in `keyring_store.rs` and `sift-e2e` appears in no other Rust source, so the literal in build A can only come from the compile-time override. In debug build B the literal sits directly before `api-key`, the order of the `SERVICE` and `USER` constants. The LTO release build does not keep that adjacency, so A rests on the first argument. This proves the compiled-in name, not that the backend is a real keychain (finding 1).

## Re-verifying a run

`$TMPDIR/sift-e2e/logs/` keeps `transcript.log` (every PASS and FAIL line plus the logs), `pids.txt`, `happy.log`, `readonly-*.log`, server and build logs. `E2E_SKIP_BUILD=1` reuses builds only when `build.stamp` (commit, build-input trees, uncommitted diff, untracked inputs) still matches; otherwise it exits 2 before clearing any logs, unless `E2E_ALLOW_STALE=1`. A later `E2E_SKIP_BUILD=1` run on the same binaries also passed (20 PASS).

## Not run / not available

- Screenshots: `screencapture -x` returned `could not create image from display` on every call, so no `05-assets/` exist. No permission was changed.
- x86_64 leg under Rosetta: not run.
- Admin-prompt cancel case: stays with the user (publish_cycle).

## Commands

```
pnpm e2e:native                      # = bash scripts/e2e-updater.sh; exit 0 on 2026-10-05, about 3 min including both builds on warm caches
E2E_SKIP_BUILD=1 bash scripts/e2e-updater.sh   # reuse builds and key while iterating on the test
node --test src/lib/stores/updater-errors.test.ts          # 26 pass; blanking 'cross-device link' makes 1 fail
cargo test --manifest-path src-tauri/Cargo.toml --lib service_defaults_to_sift
pnpm build && ! grep -rq 'e2e-log' dist
```

## Addendum: the v0.2.0 release (2026-10-05)

The local runs above used a throwaway key. Online, the release pipeline added
the one proof they could not give: that the key CI signs with is the key
installed apps trust.

| Step | Run / PR | Outcome |
|:-----|:---------|:--------|
| First rehearsal on the integration PR | #1, run 37266887344 | **FAIL, as designed.** minisign rejected the CI-built `.sig` files: key id `AFF21837544E94EA` (repo secret) ≠ `D559D24817DAAB93` (pinned in `tauri.conf.json`). Before this campaign, the release would have published and every installed copy would have rejected the update. No release had ever been published, so the maintainer rotated to a fresh pair, `1884DC5376300376`, backed up in the vault. |
| Rehearsal after rotation | #1, run 37268804646 | PASS. Both platforms verify, lipo gives arm64/x86_64 for the executable and pdfium, the `dryrun-*` draft is deleted. |
| Merge of #1 to `main` | run 37269737955 | `decide`: `release=false` (tag v0.1.4 exists). No-op, as intended. |
| Rehearsal on the release PR | #2, run 37269944747 | FAIL in a verifier unit test, not in the release. Tauri prints minisign key ids without zero padding, and the test expected 16 hex digits, so about 1 throwaway key in 16 failed. Fixed in the test (`b3a8886`); production code reads key ids from the key bytes. |
| Merge of #2 (release `57c225b`) | run 37272671132 | PASS in 563 s. v0.2.0 published as **Latest**, tag on `57c225b`, the anonymous smoke test passed. |

Independent check after publishing: the public `latest.json` (version 0.2.0,
`darwin-aarch64` and `darwin-x86_64`) and both update archives were downloaded
anonymously, as an installed app would. Both signatures verify with minisign
against the pubkey in `tauri.conf.json`, and each archive holds a top-level
`Sift.app`.

User checks:
- The 0.2.0 DMG was installed into `/Applications` by the maintainer.
- Still open:
  - Cmd+C/V in Settings, after the menu fix;
  - cancelling the administrator prompt;
  - the first in-app update from an installed 0.2.0, which needs 0.2.1 to exist.

  Check all three at the 0.2.1 release.

`main` is now protected: the four CI checks are required, branches must be up
to date, PRs are required, only merge commits are allowed, and force-push and
deletion are blocked.
