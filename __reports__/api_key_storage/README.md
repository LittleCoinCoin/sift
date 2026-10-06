# api_key_storage — Reports

Credential storage for the OCR/extraction API keys (`src-tauri/src/keyring_store.rs`).

## Documents

| Round | File | Type | Status | Topic |
|:------|:-----|:-----|:-------|:------|
| 00 | [`00-keyring_mock_backend_v0.md`](00-keyring_mock_backend_v0.md) | notice | resolved | keyring built without a platform backend, so API keys are never persisted |
| 01 | [`01-keychain_signing_measurement_v0.md`](01-keychain_signing_measurement_v0.md) (latest) | findings | decision recorded | the Keychain prompts after each update (once per stored key) under ad-hoc and self-signed signing alike; ship ad-hoc |

## Status

Resolved in v0.2.1. API keys are stored in the real macOS Keychain, and the keyring commands run off the main thread, on Tauri's blocking pool, so the dialog freezes neither the window nor the async runtime. Under ad-hoc signing, after each update macOS asks for the login keychain password once per stored key ("Always Allow"). The OCR and text-processing keys are separate items. A paid Developer ID, which gives a Team ID, is the known fix if that ever becomes unacceptable. It has not been measured.
