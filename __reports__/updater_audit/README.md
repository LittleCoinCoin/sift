# updater_audit — Reports

Reports from the auto-updater implementation (2026-05-16) and from the
`update-flow-hardening` campaign (2026-10-03 to 2026-10-05). That campaign made
the update flow work end to end and gated it behind CI and a verified release
pipeline (`__roadmap__/update-flow-hardening/`).

## Documents

| Round | File | Type | Status | Topic |
|:------|:-----|:-----|:-------|:------|
| 00 | [`00-full_install_testing_constraints_v0.md`](00-full_install_testing_constraints_v0.md) | observation | resolved | Dev-build and HTTP constraints on a full install E2E. Resolved by the agent-run native E2E (05), which uses `--debug` and `--config` overrides |
| 01 | [`01-macos_applescript_install_failure_v0.md`](01-macos_applescript_install_failure_v0.md) | notice | resolved | Raw OS errors on install failure. Resolved by `classifyInstallError`, which also covers the cross-device error seen when running from the DMG |
| 02 | [`02-linux_appimage_tempfile_mount_v0.md`](02-linux_appimage_tempfile_mount_v0.md) | notice | deferred | Linux AppImage cross-mount tempfile failure. Linux is not a release target |
| 03 | [`03-full_e2e_test_definition_v0.md`](03-full_e2e_test_definition_v0.md) | test-definition | superseded | Manual E2E plan written for v0.1.5. v0.1.5 was never released; the plan is carried out, retargeted to 0.2.0, by 04 and 05 |
| 04 | [`04-e2e_ui_results_v0.md`](04-e2e_ui_results_v0.md) (latest) | findings | done | Browser harness: 13 update-UX scenarios pass against the real store and toast component, with a negative control |
| 05 | [`05-e2e_native_results_v0.md`](05-e2e_native_results_v0.md) (latest) | findings | done | Native E2E: a signed 0.0.1 build updates itself to 0.2.0 through the real buttons, and the read-only DMG case shows the actionable message |

## Status

The update flow is proven locally by 04 and 05, and online by the v0.2.0
release (05, addendum). The first release rehearsal caught a mismatched
signing key, which was rotated before anything shipped. Still open, to check
at the 0.2.1 release: the first in-app update from an installed 0.2.0,
cancelling the administrator prompt, and Cmd+C/V in Settings.

A related product gap found during 05 is tracked separately: API keys are never
persisted because keyring has no platform backend
(`__reports__/api_key_storage/00-keyring_mock_backend_v0.md`).
