use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub const DEFAULT_SYSTEM_PROMPT_ID: &str = "default-v1";
pub const DEFAULT_SYSTEM_PROMPT_NAME: &str = "Default v1";
pub const DEFAULT_SYSTEM_PROMPT_CONTENT: &str = "You are a data extraction assistant. Given a receipt transcription, extract ONLY these fields and return a single JSON object with no other text:\n  { {schema_keys} }\nFor any field you cannot find, use an empty string.";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemPrompt {
    pub id: String,
    pub name: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    #[serde(default)]
    pub url: String,
    #[serde(alias = "model", default)]
    pub ocr_model: String,
    #[serde(default)]
    pub extraction_url: String,
    #[serde(default)]
    pub extraction_model: String,
    #[serde(default)]
    pub receipt_dir: String,
    #[serde(default, alias = "csv_columns")]
    pub json_schema_keys: Vec<String>,
    #[serde(default = "default_system_prompts")]
    pub system_prompts: Vec<SystemPrompt>,
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
