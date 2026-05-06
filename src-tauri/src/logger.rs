use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

#[non_exhaustive]
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
    /// Severity of the log entry.
    pub level: LogLevel,
    /// Human-readable log message.
    pub message: String,
    /// Unix timestamp in milliseconds when the event was emitted.
    pub timestamp: u64,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Ocr,
    Extract,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProgressEvent {
    /// Number of files fully processed so far in this job.
    pub done: u32,
    /// Total number of files in this job.
    pub total: u32,
    /// Rolling average processing time per file in milliseconds.
    pub avg_ms: f64,
    /// Identifies the job this progress event belongs to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    /// Coarse job status string (e.g. `"running"`, `"completed"`, `"failed"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Absolute path of the file currently being processed; `None` when no file is in flight.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_file: Option<String>,
    /// Pipeline phase currently active for `current_file`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<Phase>,
    /// Unix timestamp in milliseconds when processing of `current_file` began.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_started_at: Option<u64>,
    /// Unix timestamp in milliseconds when the overall job was started.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_started_at: Option<u64>,
}

pub fn emit_log(app: &AppHandle, level: LogLevel, message: impl Into<String>) {
    // CAST: u128 → u64, milliseconds since Unix epoch fit in u64 until year 584,942,417
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let _ = app.emit("log", LogEvent { level, message: message.into(), timestamp });
}
