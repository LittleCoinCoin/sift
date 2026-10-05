# Auto-Updater Full E2E — Test Definition (v0)

Date: 2026-05-16

## Executive Summary

- **What must be proven:** The complete update path — download, signature verification,
  `.app` bundle replacement, and process restart — works correctly end-to-end on macOS
  before `v0.1.5` is promoted to a published release.
- **Top risks and coverage:**

  | Risk | How addressed |
  |------|---------------|
  | Signature verification rejects valid artifact | Build with real key, verify install succeeds |
  | Progress toast never updates (Content-Length missing or zero) | Assert toast text changes during download |
  | `relaunch()` fails silently after install | Assert app version after restart |
  | Double Install click launches two concurrent downloads | Click Install twice; assert single download toast |
  | macOS replaces wrong path (dev binary vs `.app` bundle) | Use `tauri build --debug` (bundled app), not `tauri dev` |

- **Estimated volume:** 5 scenarios — 1 integration `end_to_end`, 2 integration `component`,
  2 regression. No automated test runner required; all scenarios are manual with observable
  pass/fail criteria.

---

## Scope

### In scope
- `src/lib/stores/updater.svelte.ts` — full download/install/restart flow
- `src-tauri/capabilities/default.json` — permission grants for all three IPC commands
- `scripts/serve-mock-update.py` — mock server enhanced to serve a real signed artifact
- The macOS "Sift → Check for Updates…" menu item as a manual re-trigger

### Out of scope
- CI release pipeline (tested separately when first tag is pushed)
- Linux AppImage update path (deferred — see `02-linux_appimage_tempfile_mount_v0.md`)
- macOS admin dialog on permission-denied installs (deferred — see `01-macos_applescript_install_failure_v0.md`)
- Rollback on partial install failure (Tauri plugin internals, not application code)

### Assumptions
- `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` are available
  locally as environment variables (retrieved from secure vault before running builds).
- The test machine is the target deployment architecture (aarch64 or x86_64 — build and
  test on the same machine; no cross-arch testing required).
- `/Applications` is writable without admin escalation (app installed there by the tester,
  not by a system installer).

### Trust boundaries

| Dependency | Strategy |
|---|---|
| Update manifest (`latest.json`) | Local mock server — real JSON, real signature, local artifact URL |
| `.app.tar.gz` artifact | Built locally from `main` with real signing key |
| Signature verification | Real (Tauri plugin uses the public key baked into `tauri.conf.json`) |
| GitHub releases endpoint | **Not exercised** in this test — mock server replaces it |
| macOS process restart | Real (`relaunch()` calls the OS) |

---

## Test Matrix

| Group | Scenario | Risk it covers | Tier | Setup / Data | Assertion (observable) |
|------:|----------|----------------|------|--------------|------------------------|
| Notification | Auto-check fires ~4 s after launch and shows update toast | Update detection works | regression | Old app v0.0.1 installed; mock server running; server's manifest claims v0.1.5 | "Update 0.1.5 available" toast appears within 5 s of launch; mock server terminal logs one `GET /latest.json` hit |
| Notification | Manual check via Sift → Check for Updates… re-triggers | Menu item event wiring | regression | Same as above; toast already dismissed | Second `GET /latest.json` hit logged; toast reappears |
| Download | Clicking Install shows live download progress | Progress callback wiring; Content-Length present | integration `component` | Update toast visible; mock server serves real `.app.tar.gz` with `Content-Length` header | Toast message changes from "Downloading update…" to "Downloading update… N%" at least once before reaching 100%; single download toast (no duplicates) |
| Download | Double-click Install starts only one download | Concurrent-download guard (`_downloadInProgress`) | integration `component` | Update toast visible | After two rapid Install clicks, exactly one progress toast exists; no second "Downloading update…" toast appears |
| Install + Restart | Full path: Install → progress → Restart Now → relaunch | Signature verification; bundle replacement; `relaunch()` | integration `end_to_end` | Old app v0.0.1 in `/Applications`; mock server serving signed v0.1.5 artifact | "Update ready — restart to apply" toast with Restart Now button appears after download; clicking Restart Now closes the app and reopens it; relaunched app title bar / About shows version 0.1.5 |

---

## Fixtures / Test Data Strategy

### Two builds required

**Build A — "New" artifact (v0.1.5, signed)**

```bash
# On branch: main
export TAURI_SIGNING_PRIVATE_KEY="<from vault>"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="<from vault>"
pnpm tauri build
```

Outputs (in `src-tauri/target/release/bundle/macos/`):
- `Sift.app.tar.gz` — the artifact to serve
- `Sift.app.tar.gz.sig` — the minisign signature (read by the mock server)

**Build B — "Old" app (v0.0.1, debug bundle)**

```bash
# On branch: test/updater-flow  (version=0.0.1, endpoint=http://127.0.0.1:1430/latest.json)
pnpm tauri build --debug
```

Output: `src-tauri/target/debug/bundle/macos/Sift.app`

Why `--debug`: the debug Cargo profile sets `debug_assertions = true`, which causes
`tauri-plugin-updater` to skip its HTTP-endpoint rejection gate. The result is a properly
bundled `.app` (unlike `tauri dev`) that accepts the plain-HTTP mock server without
requiring HTTPS infrastructure.

### Mock server enhancement

`scripts/serve-mock-update.py` needs one change: accept a `--artifact <path>` CLI argument.
On startup it reads `<path>.sig`, populates the manifest with:
- `version`: `0.1.5`
- `signature`: contents of `.sig` file
- `url`: `http://127.0.0.1:1430/Sift.app.tar.gz`

And adds a handler for `GET /Sift.app.tar.gz` that streams the file from disk.

### Install command

```bash
cp -r src-tauri/target/debug/bundle/macos/Sift.app /Applications/
```

(Replace any existing `/Applications/Sift.app` before the test run.)

---

## Observability Requirements

- **Mock server terminal**: must log a timestamped `GET /latest.json` line within 5 s of
  app launch (proves the check fired). Must log a `GET /Sift.app.tar.gz` line when Install
  is clicked (proves download started).
- **Toast progression**: "Downloading update… N%" must appear at least once with `N > 0`
  and `N < 100` (proves the progress callback is wired and Content-Length is present).
- **Version after restart**: the relaunched app must display v0.1.5 in its title bar or
  About panel (proves the `.app` bundle was actually replaced, not just the process restarted).

---

## Minimal Must-Run Regression Set

These two scenarios must pass before `v0.1.5` is pushed and the draft release is published:

1. **Auto-check notification** — mock server logs one `GET /latest.json`; toast appears within 5 s.
2. **Full install + restart** — "Restart Now" toast appears after download; relaunched app shows v0.1.5.

All other scenarios are useful for a first full run but are not gating.

---

## Scope Control Notes

- Consolidations: "Manual check" and "Auto-check" notification tests share setup but cover
  distinct event paths (timeout vs menu event) — kept separate.
- Double-click scenario is `component` tier, not `end_to_end`, because it tests a guard
  condition rather than a full user workflow.
- No automated assertions are proposed: Tauri's IPC and OS process replacement are not
  easily instrumented in a unit-test harness. All assertions are observable UI and log output.
