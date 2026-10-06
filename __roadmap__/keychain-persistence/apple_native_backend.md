# Apple Native Backend

**Goal**: API keys go to the real macOS Keychain, and the six keyring commands run off the main thread, so a Keychain dialog cannot freeze the window.
**Pre-conditions**:
- [ ] Working on `task/apple_native_backend`, branched from the integration tip `milestone/keychain-persistence`
- [ ] `pnpm install --frozen-lockfile && pnpm build` done in the leaf worktree (Tauri reads `frontendDist` at `cargo test` time)
**Success Gates**:
- ⬜ `(cd src-tauri && cargo test --lib)` passes, including `keyring_store::tests::backend_persists_until_delete` [run]
- ⬜ `(cd src-tauri && cargo tree -e features -i keyring)` lists `keyring feature "apple-native"` [run]
- ⬜ `git diff milestone/keychain-persistence -- src-tauri/Cargo.lock | grep '^+name = '` prints exactly one line, `+name = "security-framework"` [run]
- ⬜ `git diff --exit-code milestone/keychain-persistence -- pnpm-lock.yaml` exits 0 [run]
- ⬜ `grep -c '^#\[tauri::command(async)\]$' src-tauri/src/keyring_store.rs` prints `6` and `grep -c '^#\[tauri::command\]$' src-tauri/src/keyring_store.rs` prints `0` [static]
**References**: `__reports__/api_key_storage/01-keychain_signing_measurement_v0.md` (why, and the lockfile expectation); keyring 3.6.3 `src/lib.rs` (`pub use macos as default` with `apple-native`, `pub use mock as default` without), `src/credential.rs` (`CredentialBuilderApi::persistence`, `CredentialPersistence::{EntryOnly, UntilDelete}`); `~/.cargo/registry/src/*/tauri-macros-2.5.5/src/command/wrapper.rs` (`ExecutionContext::Blocking` vs `Async`)

## Step 1: Pin the backend with a failing test
**Goal**: A CI-safe test that fails while keyring is on its mock and passes on the real Keychain, without touching any keychain.
**Implementation Logic**:
In the `tests` module of `src-tauri/src/keyring_store.rs`, add `backend_persists_until_delete`, gated `#[cfg(target_os = "macos")]` (the CI `rust` job runs on `macos-latest`). It brings `keyring::credential::CredentialBuilderApi` into scope and asserts that `keyring::default::default_credential_builder().persistence()` matches `keyring::credential::CredentialPersistence::UntilDelete`. Use `matches!`, because the enum may not implement `PartialEq`; check that and say which you used. On today's manifest the default store is `mock`, whose persistence is `EntryOnly`, so the test fails. It reads a property of the builder and never creates a credential, so it needs no keychain access and cannot prompt. The check below matches the one-failure result, because a bare `cargo test --lib backend_persists_until_delete` exits 0 with `0 passed` when the test does not exist.
**Deliverables**: `src-tauri/src/keyring_store.rs` (`tests::backend_persists_until_delete`)
**Consistency Checks**: `cd src-tauri && cargo test --lib backend_persists_until_delete 2>&1 | grep -q 'test result: FAILED. 0 passed; 1 failed'` (expected: PASS)
**Commit**: `test(backend): assert the keychain backend persists until delete`

## Step 2: Enable the macOS Keychain backend
**Goal**: Saved API keys persist across launches and updates.
**Implementation Logic**:
1. In `src-tauri/Cargo.toml`, change `keyring = "3"` to `keyring = { version = "3", features = ["apple-native"] }`. Releases are macOS-only, so no other platform feature is added.
2. Let cargo update `src-tauri/Cargo.lock`. Expected: `keyring` gains the dependencies `security-framework 2.11.1` (keyring's iOS dependency, never built for macOS targets) and `security-framework 3.7.0`, and the existing `security-framework` references become `security-framework 3.7.0`. Any other new `[[package]]` is a finding: report it, don't commit it.
3. `keyring_store.rs` keeps its public API and the `SIFT_KEYRING_SERVICE` compile-time override unchanged.
**Deliverables**: `src-tauri/Cargo.toml` (`keyring` with `apple-native`); `src-tauri/Cargo.lock` (keyring dependency edges, `security-framework 2.11.1`)
**Consistency Checks**: `cd src-tauri && cargo test --lib` (expected: PASS)
**Commit**: `fix(backend): store API keys in the macOS Keychain instead of a mock`

## Step 3: Run keyring commands off the main thread
**Goal**: A pending Keychain password dialog blocks only the command waiting for it, not the window.
**Implementation Logic**:
Tauri 2 runs a plain `#[tauri::command]` fn on the main thread (`ExecutionContext::Blocking`). After an update, `get_api_key` blocks inside the Keychain until the user answers the password dialog, and `SettingsPanel.svelte` calls it on mount, so the window freezes. The maintainer observed this in run 1b of report 01.
Change the attribute of all six commands in `src-tauri/src/keyring_store.rs` (`set_api_key`, `get_api_key`, `delete_api_key`, `set_extraction_api_key`, `get_extraction_api_key`, `delete_extraction_api_key`) to `#[tauri::command(async)]`. That runs them on Tauri's async runtime with the same signatures, so the frontend `invoke` calls and the `generate_handler!` list in `src-tauri/src/lib.rs` stay unchanged.
If the macro rejects `(async)` on these fns, report the compiler error instead of converting them to `async fn`. The behavioural proof is the e2e leaf's `ipc alive` assertion, which the maintainer runs at L1 close.
**Deliverables**: `src-tauri/src/keyring_store.rs` (six `#[tauri::command(async)]` attributes)
**Consistency Checks**: `cd src-tauri && cargo test --lib && test "$(grep -c '^#\[tauri::command(async)\]$' src/keyring_store.rs)" = 6` (expected: PASS)
**Commit**: `fix(backend): keep the window responsive while the keychain prompts`
