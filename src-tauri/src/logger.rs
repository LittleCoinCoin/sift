use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Success,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogEvent {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProgressEvent {
    pub done: u32,
    pub total: u32,
    pub avg_ms: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_file: Option<String>,
}

pub fn emit_log(app: &AppHandle, level: LogLevel, message: impl Into<String>) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let _ = app.emit("log", LogEvent { level, message: message.into(), timestamp });
}
