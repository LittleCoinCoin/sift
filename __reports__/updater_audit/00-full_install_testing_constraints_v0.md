# Full Install E2E Testing Constraints — Observation (v0)

Date: 2026-05-16

---
type: observation
topic: updater_audit
spotted-during: auto-updater end-to-end test on test/updater-flow branch
date: 2026-05-16
domain: code
confidence: confirmed
urgency: medium
deferred-because: notification flow (toasts + buttons) was the in-scope goal; full install requires a release build and a signed artifact
---

## What Was Noticed

Two constraints block full end-to-end testing of the `downloadAndInstall()` path
using the current dev workflow:

1. **Dev binary path.** When running via `pnpm tauri dev`, the executable is
   `target/debug/sift`. The updater plugin's `extract_path_from_executable()`
   resolves to `target/debug/` rather than a `.app` bundle. Installing an update
   in this mode extracts the downloaded `.app.tar.gz` into `target/debug/`,
   corrupting the dev environment instead of performing a real install.

2. **HTTP endpoint rejected in release builds.** The test branch uses a plain-HTTP
   mock endpoint (`http://127.0.0.1:1430/latest.json`). The `tauri-plugin-updater`
   rejects plain HTTP in release builds with `InsecureTransportProtocol`. Testing
   the full install path requires a release build, which must use the production
   HTTPS endpoint — meaning a real published GitHub release.

## Context

The local test on `test/updater-flow` successfully verified the notification flow:
update check, "Update 0.1.4 available" toast, Install button, download command
dispatched. The 404 from the mock server (fake artifact URL) was the intentional
boundary. The remaining success gates — download progress percentage, "Restart Now"
toast, actual app relaunch — were not exercised.

The two constraints above are not bugs in the application code. They are
architectural properties of Tauri v2's updater plugin and cannot be worked around
without a production build.

## Location Map

- `src-tauri/tauri.conf.json` (test branch): `plugins.updater.endpoints` →
  `http://127.0.0.1:1430/latest.json`
- `src-tauri/tauri.conf.json` (milestone/auto-updater): `plugins.updater.endpoints`
  → GitHub HTTPS URL (production config, correct)
- `~/.cargo/registry/src/**/tauri-plugin-updater-2.*/src/updater.rs`:
  `extract_path_from_executable()` — walks parent dirs looking for `.app` bundle
- `~/.cargo/registry/src/**/tauri-plugin-updater-2.*/src/config.rs`:
  `validate_endpoints()` — gated on `#[cfg(not(debug_assertions))]`

## Evidence

- Local test confirmed the 404 was reached: the download command was dispatched
  and the IPC round-trip completed.
- Reading `updater.rs`: `extract_path_from_executable()` walks parent directories
  looking for a `.app` bundle; a plain debug binary (`target/debug/sift`) has no
  bundle, so the resolved path is `target/debug/`.
- Reading `config.rs`: the `InsecureTransportProtocol` error gate is wrapped in
  `#[cfg(not(debug_assertions))]` — HTTP only fails in release mode, not dev.

## Re-observation Steps

To verify the full install path:

1. Build a release bundle: `pnpm tauri build`
2. Install the `.app` to `/Applications/Sift.app`
3. Push a Git tag → GitHub Actions builds and signs the release → publishes `latest.json`
4. Lower `version` in `tauri.conf.json` to simulate an older install
5. Launch Sift; click Install on the "Update available" toast
6. Observe: progress percentage in toast, "Restart Now" toast on completion, app relaunches

## Hand-off Questions

Working theory: all application code is correct. The remaining gates depend solely
on production CI infrastructure — a signed `.app.tar.gz` and the published
`latest.json`. The first pushed and published GitHub release with
`TAURI_SIGNING_PRIVATE_KEY` in CI will satisfy both constraints automatically.

- Does the first CI release require any manual step beyond pushing a semver tag and
  publishing the draft release on GitHub?
- Is there a faster local path (e.g., `tauri signer sign` on a self-built artifact
  served via a local HTTPS server with a self-signed cert) to exercise the install
  path before the first real release?

## Scope Boundary

This report does not authorise changes to `tauri.conf.json`, the updater plugin
source, or the CI workflow. It is an orientation document for whoever runs the
full E2E test after the first published release.
