# updater_audit — Reports

Observations and notices captured during the auto-updater implementation and
local end-to-end testing session (2026-05-16).

## Documents

| File | Type | Status | Topic |
|:-----|:-----|:-------|:------|
| [`00-full_install_testing_constraints_v0.md`](00-full_install_testing_constraints_v0.md) | observation | open | Dev build and HTTP constraints blocking full install E2E test |
| [`01-macos_applescript_install_failure_v0.md`](01-macos_applescript_install_failure_v0.md) | notice | open | Raw OS error shown when macOS admin dialog is dismissed |
| [`02-linux_appimage_tempfile_mount_v0.md`](02-linux_appimage_tempfile_mount_v0.md) | notice | deferred | Linux AppImage fails on cross-mount tempfile (not current target) |

## Status

All three items were deferred from the main implementation thread. None block the
current release. `00` becomes actionable on the first published GitHub release.
`01` becomes actionable on first production install complaint. `02` becomes
actionable if Linux is added to the release matrix.
