# api_key_storage — Reports

Credential storage for the OCR/extraction API keys (`src-tauri/src/keyring_store.rs`).

## Documents

| Round | File | Type | Status | Topic |
|:------|:-----|:-----|:-------|:------|
| 00 | [`00-keyring_mock_backend_v0.md`](00-keyring_mock_backend_v0.md) | notice | resolved | keyring built without a platform backend, so API keys are never persisted |
| 01 | [`01-keychain_signing_measurement_v0.md`](01-keychain_signing_measurement_v0.md) (latest) | findings | decision recorded | the Keychain prompts once per update under ad-hoc and self-signed signing alike; ship ad-hoc |

## Status

Resolved in v0.2.1. API keys are stored in the real macOS Keychain, and the keyring commands run off the main thread, so the dialog does not freeze the window. Under ad-hoc signing, each update asks once for the login keychain password ("Always Allow"). A paid Developer ID, which gives a Team ID, is the known fix if that ever becomes unacceptable. It has not been measured.
