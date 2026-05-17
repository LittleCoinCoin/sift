# updater_audit — Reports

Observations and notices captured during the auto-updater implementation and
local end-to-end testing session (2026-05-16).

## Documents

| File | Type | Status | Topic |
|:-----|:-----|:-------|:------|
| [`00-full_install_testing_constraints_v0.md`](00-full_install_testing_constraints_v0.md) | observation | open | Dev build and HTTP constraints blocking full install E2E test |
| [`01-macos_applescript_install_failure_v0.md`](01-macos_applescript_install_failure_v0.md) | notice | open | Raw OS error shown when macOS admin dialog is dismissed |
| [`02-linux_appimage_tempfile_mount_v0.md`](02-linux_appimage_tempfile_mount_v0.md) | notice | deferred | Linux AppImage fails on cross-mount tempfile (not current target) |
| [`03-full_e2e_test_definition_v0.md`](03-full_e2e_test_definition_v0.md) | test-definition | open | Full E2E test plan for download → install → restart before v0.1.5 ships |

## Status

`03` is the gating artifact before publishing the v0.1.5 draft release. The two
scenarios in its Minimal Regression Set must pass first.

`00` describes the testing constraints that `03` works around.
`01` becomes actionable on the first production install complaint.
`02` becomes actionable if Linux is added to the release matrix.
