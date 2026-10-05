# Native Update E2E Results (v0)

Date: 2026-10-05

---
type: findings
topic: updater_audit
produced-by: task/e2e_native (`pnpm e2e:native`, `scripts/e2e-updater.sh`)
date: 2026-10-05
domain: code
confidence: confirmed (one run, exit 0; two earlier runs exposed the defects below)
---

## Headline

An installed Sift 0.0.1 (debug build, ad-hoc signed) detected 0.2.0 on a mock server, downloaded the
signed `.app.tar.gz` (5.57 MB, Content-Length present), verified it, replaced its own bundle,
and relaunched as the 0.2.0 release build. The Install and Restart Now buttons were clicked inside the real
webview by the in-app driver. The real `sift` keychain item and `/Applications` were never touched.

Four findings matter more than the pass:

| # | Finding | Evidence | Consequence |
|--:|---------|----------|-------------|
| 1 | `keyring = "3"` has no platform feature, so it uses the in-memory mock store. The app never writes a keychain item. | `cargo tree -p keyring -e features` lists only `log`. `set_api_key` then `get_api_key` in the same process returns `No matching entry found in secure storage`. The `security` tool never sees a `sift-e2e` item. | The API key is not persisted across launches. The keychain-across-identity-change question is moot until `apple-native` is enabled. Out of scope here; needs its own fix. |
| 2 | Running from a mounted disk image fails with `Update failed: Cross-device link (os error 18)`, not the readonly copy. | Case `image-realistic` below. `std::fs::rename` from `$TMPDIR` to the image gives EXDEV, which `classifyInstallError` does not match. | The leaf's readonly gate cannot be met by a plain image launch. The permission copy shows only when the temp dir is itself read-only (case `tmpdir-on-volume`). `PERMISSION_PATTERNS` probably wants `cross-device link`. |
| 3 | Exec'ing `Contents/MacOS/sift` from a script does not boot the webview. | 9 of 9 standalone trials: process alive with 0% CPU and no JS, or abort `Failed to setup app: error encountered during setup hook: unknown path`. `open -n <bundle>` booted in about 3 s every time. | The script launches with `open -n`. Cause not isolated (provenance xattr and activation were ruled out or unproven); it affects tests, not users, who launch from Finder. |
| 4 | Release builds accept `http://` endpoints with `plugins.updater.dangerousInsecureTransportProtocol: true`. | Build A sets it; its post-update check hit the mock server (3rd `GET /latest.json`). | The spec's "only debug builds" is not absolute. It lets the relaunched release app prove it does not loop. |

## Happy path (0.0.1 to 0.2.0)

Ordered `/e2e-log` (server timestamps; the 94 `Downloading update… N%` lines from 1% to 100% are collapsed):

```
13:25:55.972 booted 0.0.1
13:25:55.977 keychain set ok
13:25:55.982 keychain error|No matching entry found in secure storage     (same-process get, see finding 1)
13:25:59.985 toast Update 0.2.0 available
13:26:01.984 click available (Install)
13:26:01.996 toast Downloading update…
13:26:02.005 toast Downloading update… 1%      ... 94 lines ...   99%, 100%
13:26:03.518 toast Installing update…
13:26:03.793 toast Update ready — restart to apply
13:26:05.804 click ready (Restart Now)
13:26:06.596 booted 0.2.0
13:26:06.602 keychain error|No matching entry found in secure storage     (get at 0.2.0)
```

| Check | Result |
|-------|--------|
| Order available < click < N% (0<N<100) < installing < ready < click < booted 0.2.0 | pass (log lines 4, 5, 8, 93, 94, 95, 96) |
| `defaults read $TMPDIR/sift-e2e/Sift.app/Contents/Info CFBundleShortVersionString` | `0.2.0` |
| Installed executable sha256 equals build A's and differs from build B's | pass |
| PID before restart / after | 1721 / 1950; the old process is gone |
| Update loop after restart | none; build A re-checked (`GET /latest.json` at 13:26:10.607) and showed no toast |
| Server served `GET /artifact.tar.gz` | yes, 13:26:01.989 |
| Timings | boot 2 s; available toast 4 s after boot; download 1.5 s (15 ms pause per 64 KiB); install 0.3 s; relaunch 0.8 s after the click |

## Read-only cases (disk image, `hdiutil create -format UDRO`, mounted at `/Volumes/SiftE2E`)

| Case | Launch | Toast shown | Ready toast | Bundle / process |
|------|--------|-------------|-------------|------------------|
| `image-realistic` | default env | `Update failed: Cross-device link (os error 18)` | never | still 0.0.1, same PID |
| `tmpdir-on-volume` | `TMPDIR` on the volume | `INSTALL_MESSAGES.permission` ("Couldn't install the update: move Sift to your Applications folder ...") | never | still 0.0.1, same PID |

The "never ready" check is meaningful: the same pattern matches in the happy-path log (control assertion).

## Keychain

- Probe result: `keychain set ok`, then `keychain error|No matching entry found in secure storage` at both versions (finding 1). No system dialog appeared.
- `security find-generic-password -s sift`: absent before and after (script assertion). No `sift-e2e` item remained.
- Both binaries embed `sift-e2e` (checked before launch), and nothing is written anyway.

## Not run / not available

- Screenshots: `screencapture -x` returned `could not create image from display` on every call, so no `05-assets/` exist. No permission was changed.
- x86_64 leg under Rosetta: not run.
- Admin-prompt cancel case: stays with the user (publish_cycle).

## Commands

```
pnpm e2e:native                      # = bash scripts/e2e-updater.sh; exit 0 on 2026-10-05, about 2.5 min with warm caches
E2E_SKIP_BUILD=1 bash scripts/e2e-updater.sh   # reuse builds and key while iterating on the test
cargo test --manifest-path src-tauri/Cargo.toml --lib service_defaults_to_sift
pnpm build && ! grep -rq 'e2e-log' dist
```

Evidence of the run is kept in `$TMPDIR/sift-e2e/logs/` (`happy.log`, `readonly-*.log`, server logs, build logs).
