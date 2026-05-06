use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub const DEFAULT_SYSTEM_PROMPT_ID: &str = "default-v1";
pub const DEFAULT_SYSTEM_PROMPT_NAME: &str = "Default v1";
pub const DEFAULT_SYSTEM_PROMPT_CONTENT: &str = "You are a data extraction assistant. Given a receipt transcription, extract ONLY these fields and return a single JSON object with no other text:\n  { {schema_keys} }\nFor any field you cannot find, use an empty string.";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemPrompt {
    /// Stable identifier for this prompt (e.g. `"default-v1"`).
    pub id: String,
    /// Human-readable display name shown in the UI.
    pub name: String,
    /// Prompt text; may contain the `{schema_keys}` placeholder.
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    /// Base URL of the OCR endpoint (OpenAI-compatible).
    #[serde(default)]
    pub url: String,
    /// Model identifier for OCR requests. Deserialized from the legacy `"model"` key.
    #[serde(alias = "model", default)]
    pub ocr_model: String,
    /// Base URL of the text extraction endpoint (OpenAI-compatible).
    #[serde(default)]
    pub extraction_url: String,
    /// Model identifier for extraction requests.
    #[serde(default)]
    pub extraction_model: String,
    /// Legacy single receipt directory; migrated to `receipt_dirs` on first load.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub receipt_dir: String,
    /// Ordered list of directories to scan for receipt files.
    #[serde(default)]
    pub receipt_dirs: Vec<String>,
    /// Ordered list of field names to extract. Deserialized from the legacy `"csv_columns"` key.
    #[serde(default, alias = "csv_columns")]
    pub json_schema_keys: Vec<String>,
    /// All configured system prompts; defaults to a single built-in prompt.
    #[serde(default = "default_system_prompts")]
    pub system_prompts: Vec<SystemPrompt>,
    /// `id` of the currently active system prompt from `system_prompts`.
    #[serde(default = "default_active_system_prompt_id")]
    pub active_system_prompt_id: String,
}

fn default_system_prompts() -> Vec<SystemPrompt> {
    vec![SystemPrompt {
        id: DEFAULT_SYSTEM_PROMPT_ID.into(),
        name: DEFAULT_SYSTEM_PROMPT_NAME.into(),
        content: DEFAULT_SYSTEM_PROMPT_CONTENT.into(),
    }]
}

fn default_active_system_prompt_id() -> String {
    DEFAULT_SYSTEM_PROMPT_ID.into()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            url: String::new(),
            ocr_model: String::new(),
            extraction_url: String::new(),
            extraction_model: String::new(),
            receipt_dir: String::new(),
            receipt_dirs: Vec::new(),
            json_schema_keys: Vec::new(),
            system_prompts: default_system_prompts(),
            active_system_prompt_id: default_active_system_prompt_id(),
        }
    }
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|d: PathBuf| d.join("settings.json"))
        .map_err(|e: tauri::Error| e.to_string())
}

/// Load settings from the app data directory, applying field migrations.
///
/// Returns default settings if the file does not yet exist.
///
/// # Errors
/// Returns an error string if the app data path cannot be resolved, the file
/// cannot be read, or the JSON cannot be deserialized.
#[tauri::command]
pub fn get_settings(app: AppHandle) -> Result<Settings, String> {
    let path = settings_path(&app)?;
    if !path.exists() {
        return Ok(Settings::default());
    }
    let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut settings: Settings = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    if settings.receipt_dirs.is_empty() && !settings.receipt_dir.is_empty() {
        settings.receipt_dirs.push(std::mem::take(&mut settings.receipt_dir));
    }
    Ok(settings)
}

/// Persist settings to the app data directory.
///
/// # Errors
/// Returns an error string if the app data path cannot be resolved, the parent
/// directory cannot be created, JSON serialization fails, or the write fails.
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
            extraction_url: "http://localhost:8000".into(),
            extraction_model: "mistral".into(),
            ..Settings::default()
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.url, s.url);
        assert_eq!(back.ocr_model, s.ocr_model);
        assert_eq!(back.extraction_url, s.extraction_url);
        assert_eq!(back.extraction_model, s.extraction_model);
        assert_eq!(back.active_system_prompt_id, DEFAULT_SYSTEM_PROMPT_ID);
        assert_eq!(back.system_prompts.len(), 1);
    }

    #[test]
    fn settings_default_has_default_system_prompt() {
        let s = Settings::default();
        assert!(s.url.is_empty());
        assert!(s.ocr_model.is_empty());
        assert!(s.extraction_url.is_empty());
        assert!(s.extraction_model.is_empty());
        assert!(s.json_schema_keys.is_empty());
        assert_eq!(s.active_system_prompt_id, DEFAULT_SYSTEM_PROMPT_ID);
        assert_eq!(s.system_prompts.len(), 1);
        let prompt = &s.system_prompts[0];
        assert_eq!(prompt.id, DEFAULT_SYSTEM_PROMPT_ID);
        assert_eq!(prompt.name, DEFAULT_SYSTEM_PROMPT_NAME);
        assert!(prompt.content.contains("{schema_keys}"));
    }

    #[test]
    fn settings_alias_migration_model_field() {
        let json = r#"{"url":"http://localhost","model":"lightonocr","receipt_dir":"/tmp","csv_columns":[]}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.ocr_model, "lightonocr");
        assert!(s.extraction_model.is_empty());
        assert!(s.json_schema_keys.is_empty());
    }

    #[test]
    fn settings_receipt_dir_migrates_to_receipt_dirs() {
        let json = r#"{"url":"http://localhost","receipt_dir":"/tmp/receipts"}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        // simulate get_settings migration branch
        if s.receipt_dirs.is_empty() && !s.receipt_dir.is_empty() {
            s.receipt_dirs.push(std::mem::take(&mut s.receipt_dir));
        }
        assert_eq!(s.receipt_dirs, vec!["/tmp/receipts"]);
        // skip_serializing_if omits the legacy field on save
        let out = serde_json::to_string(&s).unwrap();
        assert!(out.contains("receipt_dirs"));
        assert!(!out.contains(r#""receipt_dir""#));
    }

    #[test]
    fn settings_csv_columns_alias_migrates_to_json_schema_keys() {
        let json = r#"{"url":"http://localhost","csv_columns":["vendor","total","date"]}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.json_schema_keys, vec!["vendor", "total", "date"]);
    }

    #[test]
    fn settings_json_schema_keys_roundtrip() {
        let s = Settings {
            json_schema_keys: vec!["vendor".into(), "amount".into()],
            ..Settings::default()
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("json_schema_keys"));
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.json_schema_keys, s.json_schema_keys);
    }

    #[test]
    fn settings_missing_system_prompts_gets_default() {
        let json = r#"{"url":"http://localhost"}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.system_prompts.len(), 1);
        assert_eq!(s.active_system_prompt_id, DEFAULT_SYSTEM_PROMPT_ID);
    }

    #[test]
    fn settings_custom_system_prompts_preserved() {
        let custom = Settings {
            system_prompts: vec![
                SystemPrompt {
                    id: "v1".into(),
                    name: "First".into(),
                    content: "prompt one".into(),
                },
                SystemPrompt {
                    id: "v2".into(),
                    name: "Second".into(),
                    content: "prompt two with {schema_keys}".into(),
                },
            ],
            active_system_prompt_id: "v2".into(),
            ..Settings::default()
        };
        let json = serde_json::to_string(&custom).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.system_prompts.len(), 2);
        assert_eq!(back.active_system_prompt_id, "v2");
        assert_eq!(back.system_prompts[1].content, "prompt two with {schema_keys}");
    }
}
