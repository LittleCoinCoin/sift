use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProcessingStatus {
    Unprocessed,
    Processing,
    Processed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptEntry {
    /// Absolute path of the source receipt file on disk. Used as the index key.
    pub source_path: String,
    /// Current processing state of the receipt.
    pub status: ProcessingStatus,
    /// Extracted key-value fields; `None` until the receipt has been processed.
    pub fields: Option<HashMap<String, String>>,
    /// Last-modified time of the source file as Unix seconds; `0` if unavailable.
    pub source_mtime: u64,
}

pub type ReceiptIndex = HashMap<String, ReceiptEntry>;

fn index_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|d| d.join("receipt_index.json"))
        .map_err(|e| e.to_string())
}

/// Load the persisted receipt index from the app data directory.
///
/// Returns an empty index if the file does not yet exist.
///
/// # Errors
/// Returns an error string if the app data path cannot be resolved, the file
/// cannot be read, or the JSON cannot be deserialized.
pub async fn load_index(app: &AppHandle) -> Result<ReceiptIndex, String> {
    let path = index_path(app)?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let data = tokio::fs::read_to_string(&path).await.map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

/// Persist the receipt index to the app data directory atomically via a `.tmp` rename.
///
/// # Errors
/// Returns an error string if the app data path cannot be resolved, the parent
/// directory cannot be created, JSON serialization fails, or the write or rename fails.
pub async fn save_index(app: &AppHandle, index: &ReceiptIndex) -> Result<(), String> {
    let path = index_path(app)?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    let data = serde_json::to_string_pretty(index).map_err(|e| e.to_string())?;
    tokio::fs::write(&tmp, data).await.map_err(|e| e.to_string())?;
    tokio::fs::rename(&tmp, &path).await.map_err(|e| e.to_string())
}

/// Replace the stored fields of a processed receipt entry.
///
/// # Errors
/// Returns an error string if no entry exists for `source_path`, or if the
/// entry's status is not [`ProcessingStatus::Processed`].
pub fn update_entry_fields(
    index: &mut ReceiptIndex,
    source_path: &str,
    fields: HashMap<String, String>,
) -> Result<(), String> {
    let entry = index
        .get_mut(source_path)
        .ok_or_else(|| format!("No receipt entry for {}", source_path))?;
    if entry.status != ProcessingStatus::Processed {
        return Err(format!(
            "Cannot edit fields for {}: receipt is not in Processed state",
            source_path
        ));
    }
    entry.fields = Some(fields);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_entry_roundtrip() {
        let mut fields = HashMap::new();
        fields.insert("vendor".to_string(), "ACME".to_string());
        let entry = ReceiptEntry {
            source_path: "/tmp/receipt.jpg".to_string(),
            status: ProcessingStatus::Processed,
            fields: Some(fields),
            source_mtime: 1_700_000_000,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let back: ReceiptEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.source_path, entry.source_path);
        assert_eq!(back.status, ProcessingStatus::Processed);
        assert_eq!(back.source_mtime, entry.source_mtime);
        assert_eq!(back.fields.unwrap().get("vendor").unwrap(), "ACME");
    }

    #[test]
    fn processing_status_roundtrip() {
        for status in [
            ProcessingStatus::Unprocessed,
            ProcessingStatus::Processing,
            ProcessingStatus::Processed,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let back: ProcessingStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, status);
        }
    }

    #[test]
    fn empty_index_roundtrip() {
        let index: ReceiptIndex = HashMap::new();
        let json = serde_json::to_string(&index).unwrap();
        let back: ReceiptIndex = serde_json::from_str(&json).unwrap();
        assert!(back.is_empty());
    }

    fn processed_entry(path: &str) -> ReceiptEntry {
        let mut fields = HashMap::new();
        fields.insert("vendor".to_string(), "ACME".to_string());
        fields.insert("total".to_string(), "10.00".to_string());
        ReceiptEntry {
            source_path: path.to_string(),
            status: ProcessingStatus::Processed,
            fields: Some(fields),
            source_mtime: 1_700_000_000,
        }
    }

    #[test]
    fn update_entry_fields_replaces_fields_on_processed_entry() {
        let mut index: ReceiptIndex = HashMap::new();
        index.insert("/r/a.jpg".to_string(), processed_entry("/r/a.jpg"));

        let mut new_fields = HashMap::new();
        new_fields.insert("vendor".to_string(), "Edited".to_string());
        new_fields.insert("total".to_string(), "12.50".to_string());

        update_entry_fields(&mut index, "/r/a.jpg", new_fields).unwrap();

        let entry = index.get("/r/a.jpg").unwrap();
        let stored = entry.fields.as_ref().unwrap();
        assert_eq!(stored.get("vendor").unwrap(), "Edited");
        assert_eq!(stored.get("total").unwrap(), "12.50");
        assert_eq!(entry.status, ProcessingStatus::Processed);
        assert_eq!(entry.source_mtime, 1_700_000_000);
    }

    #[test]
    fn update_entry_fields_does_not_touch_other_entries() {
        let mut index: ReceiptIndex = HashMap::new();
        index.insert("/r/a.jpg".to_string(), processed_entry("/r/a.jpg"));
        index.insert("/r/b.jpg".to_string(), processed_entry("/r/b.jpg"));

        let mut new_fields = HashMap::new();
        new_fields.insert("vendor".to_string(), "Edited".to_string());

        update_entry_fields(&mut index, "/r/a.jpg", new_fields).unwrap();

        let other = index.get("/r/b.jpg").unwrap();
        assert_eq!(other.fields.as_ref().unwrap().get("vendor").unwrap(), "ACME");
    }

    #[test]
    fn update_entry_fields_errors_when_entry_missing() {
        let mut index: ReceiptIndex = HashMap::new();
        let err = update_entry_fields(&mut index, "/r/missing.jpg", HashMap::new()).unwrap_err();
        assert!(err.contains("/r/missing.jpg"));
    }

    #[test]
    fn update_entry_fields_errors_when_entry_unprocessed() {
        let mut index: ReceiptIndex = HashMap::new();
        let mut entry = processed_entry("/r/a.jpg");
        entry.status = ProcessingStatus::Unprocessed;
        index.insert("/r/a.jpg".to_string(), entry);

        let err = update_entry_fields(&mut index, "/r/a.jpg", HashMap::new()).unwrap_err();
        assert!(err.contains("Processed"));
    }
}
