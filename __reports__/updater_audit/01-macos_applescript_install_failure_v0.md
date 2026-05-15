# macOS Admin Prompt on Update Install — Notice (v0)

Date: 2026-05-16

---
type: notice
title: surface AppleScript install failure with actionable message
tag: gap
effort: S
reversibility: easy
evidence: verified
spotted-during: auto-updater flow audit, tracing downloadAndInstall() on macOS
date: 2026-05-16
status: open
---

## TL;DR

- Tauri escalates to AppleScript when `rename()` is permission-denied; user dismissal
  shows a raw OS error string in the toast.
- First production install on a system where the `.app` directory is not user-writable
  will expose this immediately.
- No hook to intercept the OS dialog; fix is error-string classification in the catch block.

## Context

When `downloadAndInstall()` fails to replace the `.app` bundle via `std::fs::rename()`
(e.g., `/Applications` owned by root), the Tauri plugin escalates via AppleScript:

```applescript
do shell script "rm -rf '{old}' && mv -f '{new}' '{old}'" with administrator privileges
```

If the user dismisses the system dialog, the plugin returns an error. The current
catch block in `startDownload()` shows `String(err)` directly — which at this point
is a raw macOS error code (`-128 UserCancelled` or `PermissionDenied (os error 1)`).
Additionally, if the rename had already started before the dialog appeared, the old
`.app` backup in a temp directory is left unreferenced with no rollback.

## Scope Delta

- `src/lib/stores/updater.svelte.ts` → `startDownload()` catch block: detect
  permission/AppleScript error strings and replace with user-readable copy
  ("Installation requires administrator access — please approve the system dialog
  and try again").
- Optionally: a pre-flight check before download begins, warning the user that an
  admin prompt may appear, rather than surfacing it as a failure after the fact.

## Accept / Decline

| | Accept | Decline |
|---|---|---|
| **Benefit** | Users see an actionable instruction rather than a raw OS error code; eliminates the most likely first-impression failure on a standard `/Applications` install | The current catch block already shows `String(err)`, which is a strict improvement over the previous silent swallow |
| **Cost** | Requires string-matching on Tauri plugin error output — not a stable API; may need updating on plugin upgrades | First production install failure on a permission-restricted system surfaces an unreadable error; likely to generate a support report |

## If Accepted — Next Step

Open a dedicated thread. Update the `startDownload` catch block with a macOS-specific
error classifier; add a short integration note to `__reports__/updater_audit/README.md`.

## If Declined — Next Step

Archive. Revisit after first production release — assess frequency of install
failures before deciding whether the classifier is worth the fragility cost.
