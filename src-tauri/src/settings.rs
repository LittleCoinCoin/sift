use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Settings {
    pub url: String,
    #[serde(alias = "model", default)]
    pub ocr_model: String,
    #[serde(default)]
    pub extraction_model: String,
    #[serde(default)]
    pub receipt_dir: String,
    #[serde(default)]
    pub csv_columns: Vec<String>,
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|d: PathBuf| d.join("settings.json"))
        .map_err(|e: tauri::Error| e.to_string())
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Result<Settings, String> {
    let path = settings_path(&app)?;
    if !path.exists() {
        return Ok(Settings::default());
    }
    let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    let path = settings_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_roundtrip() {
        let s = Settings {
            url: "http://localhost:11434".into(),
            ocr_model: "llama3".into(),
            extraction_model: "mistral".into(),
            ..Settings::default()
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.url, s.url);
        assert_eq!(back.ocr_model, s.ocr_model);
        assert_eq!(back.extraction_model, s.extraction_model);
    }

    #[test]
    fn settings_default_is_empty() {
        let s = Settings::default();
        assert!(s.url.is_empty());
        assert!(s.ocr_model.is_empty());
        assert!(s.extraction_model.is_empty());
    }

    #[test]
    fn settings_alias_migration() {
        let json = r#"{"url":"http://localhost","model":"lightonocr","receipt_dir":"/tmp","csv_columns":[]}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.ocr_model, "lightonocr");
        assert!(s.extraction_model.is_empty());
    }
}
