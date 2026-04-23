use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProcessingStatus {
    Unprocessed,
    Processing,
    Processed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptEntry {
    pub source_path: String,
    pub status: ProcessingStatus,
    pub fields: Option<HashMap<String, String>>,
    pub source_mtime: u64,
}

pub type ReceiptIndex = HashMap<String, ReceiptEntry>;

fn index_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|d| d.join("receipt_index.json"))
        .map_err(|e| e.to_string())
}

pub async fn load_index(app: &AppHandle) -> Result<ReceiptIndex, String> {
    let path = index_path(app)?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let data = tokio::fs::read_to_string(&path).await.map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

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
}
