# Linux Auto-Update Fails on Cross-Mount AppImage — Notice (v0)

Date: 2026-05-16

---
type: notice
title: Linux AppImage update blocked by cross-mount tempfile constraint
tag: gap
effort: S (documentation) / M (pre-flight check)
reversibility: easy
evidence: verified
spotted-during: auto-updater flow audit, tracing Tauri plugin tempdir selection logic
date: 2026-05-16
status: deferred
---

## TL;DR

- Tauri updater requires the temp dir and the app directory to share the same
  filesystem mount; AppImages on external or non-root partitions fail silently with
  `TempDirNotOnSameMountPoint`.
- Not a current concern — macOS is the only active deployment target.
- If Linux support is added, users on non-standard installs get no error message.

## Scope Delta

- `scripts/serve-mock-update.py` or a `TESTING.md`: document that Linux testing
  requires the AppImage and `/tmp` to be on the same mount point.
- Optionally: a Rust command (`check_update_preconditions`) exposed to the frontend
  so `checkForUpdates()` can warn users before attempting a download that will fail.

## Accept / Decline

| | Accept | Decline |
|---|---|---|
| **Benefit** | Linux users on non-standard installs get an actionable message; prevents a confusing silent failure on first update attempt | Linux is not on the current target list; pre-flight code adds complexity for a hypothetical user base |
| **Cost** | Requires a Rust-side command (mount-point comparison is not available from JS); adds a JS→Rust round-trip on every update check | If Linux support is added later, this surfaces as a bug report on first install attempt — straightforward to diagnose and fix at that point |

## If Accepted — Next Step

Defer until Linux is added to the release matrix. At that point: open a dedicated
thread, implement a `check_update_preconditions` Rust command, and wire it into
`initUpdaterStore`.

## If Declined — Next Step

Archive. Revisit when a Linux target is added to `.github/workflows/release.yml`.
