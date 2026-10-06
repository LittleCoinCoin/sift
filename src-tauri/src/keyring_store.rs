use keyring::Entry;

// Compile-time override so e2e builds (SIFT_KEYRING_SERVICE=sift-e2e) can never
// read or write the keychain items of the real app. Production leaves it unset.
const SERVICE: &str = match option_env!("SIFT_KEYRING_SERVICE") {
    Some(service) => service,
    None => "sift",
};
const USER: &str = "api-key";
const EXTRACTION_USER: &str = "extraction-api-key";

// A Keychain call can block until the user answers an authorization dialog
// (once per stored key after each update of an ad-hoc signed build). Run it on
// the blocking pool so it holds neither the main thread nor an async worker.
async fn on_blocking_pool<T: Send + 'static>(
    f: impl FnOnce() -> keyring::Result<T> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_api_key(key: String) -> Result<(), String> {
    on_blocking_pool(move || Entry::new(SERVICE, USER)?.set_password(&key)).await
}

#[tauri::command]
pub async fn get_api_key() -> Result<String, String> {
    on_blocking_pool(|| Entry::new(SERVICE, USER)?.get_password()).await
}

#[tauri::command]
pub async fn delete_api_key() -> Result<(), String> {
    on_blocking_pool(|| Entry::new(SERVICE, USER)?.delete_credential()).await
}

#[tauri::command]
pub async fn set_extraction_api_key(key: String) -> Result<(), String> {
    on_blocking_pool(move || Entry::new(SERVICE, EXTRACTION_USER)?.set_password(&key)).await
}

#[tauri::command]
pub async fn get_extraction_api_key() -> Result<String, String> {
    on_blocking_pool(|| Entry::new(SERVICE, EXTRACTION_USER)?.get_password()).await
}

#[tauri::command]
pub async fn delete_extraction_api_key() -> Result<(), String> {
    on_blocking_pool(|| Entry::new(SERVICE, EXTRACTION_USER)?.delete_credential()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_defaults_to_sift() {
        match option_env!("SIFT_KEYRING_SERVICE") {
            None => assert_eq!(SERVICE, "sift"),
            Some(service) => assert_eq!(SERVICE, service),
        }
    }

    #[test]
    fn entry_construction_succeeds() {
        // Verify we can construct an Entry without panicking.
        let entry = Entry::new(SERVICE, USER);
        assert!(entry.is_ok());
    }

    #[test]
    fn extraction_entry_construction_succeeds() {
        let entry = Entry::new(SERVICE, EXTRACTION_USER);
        assert!(entry.is_ok());
    }

    // Pins the backend: keyring's mock store reports EntryOnly, the macOS
    // Keychain reports UntilDelete. Reads a builder property only; it never
    // creates a credential, so it cannot touch or prompt the keychain.
    #[cfg(target_os = "macos")]
    #[test]
    fn backend_persists_until_delete() {
        use keyring::credential::CredentialPersistence;

        let builder = keyring::default::default_credential_builder();
        assert!(matches!(
            builder.persistence(),
            CredentialPersistence::UntilDelete
        ));
    }
}
