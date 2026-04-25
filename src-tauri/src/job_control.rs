use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::sync::watch;

// === Public types ===

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Idle,
    Running,
    Paused,
    Resuming,
    Cancelling,
    Cancelled,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileOutcomeStatus {
    Processed,
    Abandoned,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOutcome {
    pub path: String,
    pub status: FileOutcomeStatus,
    pub in_flight: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub file_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobConfig {
    pub files: Vec<FileEntry>,
    pub api_url: String,
    pub api_key: String,
    pub ocr_model: String,
    pub extraction_url: String,
    pub extraction_api_key: String,
    pub extraction_model: String,
    pub active_system_prompt: String,
    pub json_schema_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSummary {
    pub job_id: String,
    pub status: JobStatus,
    pub total: u32,
    pub processed: u32,
    pub abandoned: u32,
    pub failed: u32,
    pub files: Vec<FileOutcome>,
}

// === Internal types ===

#[derive(Debug, Clone, PartialEq)]
enum ControlSignal {
    Run,
    Pause,
    Cancel,
}

#[derive(Clone, Serialize)]
struct JobStatusEvent {
    job_id: String,
    status: JobStatus,
}

// === Job handle ===

pub struct JobHandle {
    pub status: Arc<Mutex<JobStatus>>,
    pub job_id: String,
    app: AppHandle,
    ctrl_tx: watch::Sender<ControlSignal>,
    processed: Arc<AtomicU32>,
    abandoned: Arc<AtomicU32>,
    failed: Arc<AtomicU32>,
    total: u32,
    outcomes: Arc<Mutex<Vec<FileOutcome>>>,
    _task: tokio::task::JoinHandle<()>,
}

impl JobHandle {
    pub fn pause(&self) -> Result<(), String> {
        let mut status = self.status.lock().map_err(|e| e.to_string())?;
        if *status != JobStatus::Running {
            return Err(format!("Job is not running (status: {:?})", *status));
        }
        *status = JobStatus::Paused;
        self.ctrl_tx.send(ControlSignal::Pause).ok();
        // Orchestrator confirms with job_status: Paused when it enters the wait loop.
        Ok(())
    }

    pub fn resume(&self) -> Result<(), String> {
        let mut status = self.status.lock().map_err(|e| e.to_string())?;
        if *status != JobStatus::Paused {
            return Err(format!("Job is not paused (status: {:?})", *status));
        }
        *status = JobStatus::Resuming;
        let _ = self.app.emit("job_status", JobStatusEvent {
            job_id: self.job_id.clone(),
            status: JobStatus::Resuming,
        });
        self.ctrl_tx.send(ControlSignal::Run).ok();
        Ok(())
    }

    pub fn cancel(&self) -> Result<(), String> {
        let mut status = self.status.lock().map_err(|e| e.to_string())?;
        match *status {
            JobStatus::Running | JobStatus::Paused => {}
            _ => return Err(format!("Job cannot be cancelled (status: {:?})", *status)),
        }
        *status = JobStatus::Cancelling;
        let _ = self.app.emit("job_status", JobStatusEvent {
            job_id: self.job_id.clone(),
            status: JobStatus::Cancelling,
        });
        self.ctrl_tx.send(ControlSignal::Cancel).ok();
        Ok(())
    }

    pub fn summary(&self) -> JobSummary {
        JobSummary {
            job_id: self.job_id.clone(),
            status: self.status.lock().unwrap().clone(),
            total: self.total,
            processed: self.processed.load(Ordering::Relaxed),
            abandoned: self.abandoned.load(Ordering::Relaxed),
            failed: self.failed.load(Ordering::Relaxed),
            files: self.outcomes.lock().unwrap().clone(),
        }
    }
}

// === Registry ===

pub struct JobRegistry(pub Mutex<HashMap<String, JobHandle>>);

impl Default for JobRegistry {
    fn default() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

// === ID generation ===

static JOB_COUNTER: AtomicU64 = AtomicU64::new(1);

fn new_job_id() -> String {
    format!("job_{}", JOB_COUNTER.fetch_add(1, Ordering::Relaxed))
}

// === Spawn entry point ===

pub fn spawn_job(app: AppHandle, config: JobConfig) -> JobHandle {
    let job_id = new_job_id();
    let total = config.files.len() as u32;
    let status = Arc::new(Mutex::new(JobStatus::Running));
    let processed = Arc::new(AtomicU32::new(0));
    let abandoned = Arc::new(AtomicU32::new(0));
    let failed = Arc::new(AtomicU32::new(0));
    let outcomes: Arc<Mutex<Vec<FileOutcome>>> = Arc::new(Mutex::new(Vec::new()));
    let (ctrl_tx, ctrl_rx) = watch::channel(ControlSignal::Run);

    let task = tokio::spawn(run_job(
        app.clone(),
        job_id.clone(),
        config,
        Arc::clone(&status),
        ctrl_rx,
        Arc::clone(&processed),
        Arc::clone(&abandoned),
        Arc::clone(&failed),
        Arc::clone(&outcomes),
    ));

    JobHandle {
        status,
        job_id,
        app,
        ctrl_tx,
        processed,
        abandoned,
        failed,
        total,
        outcomes,
        _task: task,
    }
}

// === Orchestrator ===

async fn run_job(
    app: AppHandle,
    job_id: String,
    config: JobConfig,
    status: Arc<Mutex<JobStatus>>,
    mut ctrl_rx: watch::Receiver<ControlSignal>,
    processed_count: Arc<AtomicU32>,
    abandoned_count: Arc<AtomicU32>,
    failed_count: Arc<AtomicU32>,
    outcomes: Arc<Mutex<Vec<FileOutcome>>>,
) {
    use crate::logger::{emit_log, LogLevel, ProgressEvent};
    use crate::receipt_index::{load_index, save_index, ProcessingStatus, ReceiptEntry};
    use crate::scan::ReceiptFile;

    let total = config.files.len() as u32;
    let mut elapsed_sum = 0.0f64;

    // Mark initial value as seen so ctrl_rx.changed() waits for a real change.
    drop(ctrl_rx.borrow_and_update());

    let _ = app.emit("job_status", JobStatusEvent {
        job_id: job_id.clone(),
        status: JobStatus::Running,
    });

    'file_loop: for (idx, file_entry) in config.files.iter().enumerate() {
        // --- pause/cancel checkpoint ---
        loop {
            let sig = ctrl_rx.borrow_and_update().clone();
            match sig {
                ControlSignal::Cancel => {
                    // Append this and all remaining files as unstarted-abandoned.
                    for remaining in &config.files[idx..] {
                        outcomes.lock().unwrap().push(FileOutcome {
                            path: remaining.path.clone(),
                            status: FileOutcomeStatus::Abandoned,
                            in_flight: false,
                        });
                        abandoned_count.fetch_add(1, Ordering::Relaxed);
                    }
                    emit_terminal_cancelled(&app, &job_id, &status, total, &processed_count, &abandoned_count, &failed_count, &outcomes);
                    return;
                }
                ControlSignal::Pause => {
                    let _ = app.emit("job_status", JobStatusEvent {
                        job_id: job_id.clone(),
                        status: JobStatus::Paused,
                    });
                    if ctrl_rx.changed().await.is_err() {
                        return;
                    }
                    let next = ctrl_rx.borrow_and_update().clone();
                    if next == ControlSignal::Cancel {
                        // Append this and all remaining files as unstarted-abandoned.
                        for remaining in &config.files[idx..] {
                            outcomes.lock().unwrap().push(FileOutcome {
                                path: remaining.path.clone(),
                                status: FileOutcomeStatus::Abandoned,
                                in_flight: false,
                            });
                            abandoned_count.fetch_add(1, Ordering::Relaxed);
                        }
                        emit_terminal_cancelled(&app, &job_id, &status, total, &processed_count, &abandoned_count, &failed_count, &outcomes);
                        return;
                    }
                    let _ = app.emit("job_status", JobStatusEvent {
                        job_id: job_id.clone(),
                        status: JobStatus::Running,
                    });
                }
                ControlSignal::Run => break,
            }
        }

        let path = file_entry.path.clone();
        let receipt_file = match file_entry.file_type.as_str() {
            "pdf" => ReceiptFile::Pdf(PathBuf::from(&path)),
            _ => ReceiptFile::Image(PathBuf::from(&path)),
        };

        let done_so_far = processed_count.load(Ordering::Relaxed);
        let avg_so_far = if done_so_far > 0 { elapsed_sum / done_so_far as f64 } else { 0.0 };

        let _ = app.emit("progress", ProgressEvent {
            done: done_so_far,
            total,
            avg_ms: avg_so_far,
            job_id: Some(job_id.clone()),
            status: Some("running".to_string()),
            current_file: Some(path.clone()),
        });

        emit_log(&app, LogLevel::Info, format!("Processing {}", path));

        let start = std::time::Instant::now();

        tokio::select! {
            result = crate::receipt::process_receipt(
                receipt_file,
                &config.api_url,
                &config.api_key,
                &config.ocr_model,
                &config.extraction_url,
                &config.extraction_api_key,
                &config.extraction_model,
                &config.active_system_prompt,
                &config.json_schema_keys,
            ) => {
                let elapsed_ms = start.elapsed().as_millis() as f64;
                match result {
                    Ok(record) => {
                        match load_index(&app).await {
                            Ok(mut index) => {
                                let entry = index.entry(path.clone()).or_insert_with(|| ReceiptEntry {
                                    source_path: path.clone(),
                                    status: ProcessingStatus::Unprocessed,
                                    fields: None,
                                    source_mtime: 0,
                                });
                                entry.status = ProcessingStatus::Processed;
                                entry.fields = Some(record.fields.clone());
                                let _ = save_index(&app, &index).await;
                            }
                            Err(e) => {
                                emit_log(&app, LogLevel::Error, format!("Failed to load receipt index: {}", e));
                            }
                        }

                        let done = processed_count.fetch_add(1, Ordering::Relaxed) + 1;
                        elapsed_sum += elapsed_ms;
                        let avg_ms = elapsed_sum / done as f64;

                        outcomes.lock().unwrap().push(FileOutcome {
                            path: path.clone(),
                            status: FileOutcomeStatus::Processed,
                            in_flight: false,
                        });

                        emit_log(&app, LogLevel::Success, format!("Processed {}", path));
                        let _ = app.emit("progress", ProgressEvent {
                            done,
                            total,
                            avg_ms,
                            job_id: Some(job_id.clone()),
                            status: Some("running".to_string()),
                            current_file: Some(path.clone()),
                        });
                    }
                    Err(e) => {
                        failed_count.fetch_add(1, Ordering::Relaxed);
                        outcomes.lock().unwrap().push(FileOutcome {
                            path: path.clone(),
                            status: FileOutcomeStatus::Failed,
                            in_flight: false,
                        });
                        emit_log(&app, LogLevel::Error, format!("Failed to process {}: {}", path, e));
                    }
                }
            }
            _ = ctrl_rx.changed() => {
                // In-flight file dropped here (cancel-on-drop semantics for reqwest).
                emit_log(&app, LogLevel::Warn, format!("Abandoned {} — not persisted to index", path));
                abandoned_count.fetch_add(1, Ordering::Relaxed);
                outcomes.lock().unwrap().push(FileOutcome {
                    path: path.clone(),
                    status: FileOutcomeStatus::Abandoned,
                    in_flight: true,
                });
                let sig = ctrl_rx.borrow_and_update().clone();
                if sig == ControlSignal::Cancel {
                    // Collect all remaining unstarted files (idx+1 onward).
                    for remaining in &config.files[idx + 1..] {
                        outcomes.lock().unwrap().push(FileOutcome {
                            path: remaining.path.clone(),
                            status: FileOutcomeStatus::Abandoned,
                            in_flight: false,
                        });
                        abandoned_count.fetch_add(1, Ordering::Relaxed);
                    }
                    emit_terminal_cancelled(&app, &job_id, &status, total, &processed_count, &abandoned_count, &failed_count, &outcomes);
                    return;
                }
                // Pause: outer loop will wait at checkpoint on next iteration.
                continue 'file_loop;
            }
        }
    }

    // All files iterated — determine final status.
    let processed = processed_count.load(Ordering::Relaxed);
    let failed = failed_count.load(Ordering::Relaxed);
    let final_status = if processed == 0 && failed > 0 {
        JobStatus::Failed
    } else {
        JobStatus::Completed
    };
    *status.lock().unwrap() = final_status.clone();

    let avg_ms = if processed > 0 { elapsed_sum / processed as f64 } else { 0.0 };
    let completion_status_str = match &final_status {
        JobStatus::Failed => "failed",
        _ => "completed",
    };
    let _ = app.emit("progress", ProgressEvent {
        done: processed,
        total,
        avg_ms,
        job_id: Some(job_id.clone()),
        status: Some(completion_status_str.to_string()),
        current_file: None,
    });

    let summary = JobSummary {
        job_id: job_id.clone(),
        status: final_status.clone(),
        total,
        processed,
        abandoned: abandoned_count.load(Ordering::Relaxed),
        failed,
        files: outcomes.lock().unwrap().clone(),
    };
    let _ = app.emit("job_status", JobStatusEvent {
        job_id: job_id.clone(),
        status: final_status,
    });
    let _ = app.emit("job_done", &summary);
}

fn emit_terminal_cancelled(
    app: &AppHandle,
    job_id: &str,
    status: &Arc<Mutex<JobStatus>>,
    total: u32,
    processed_count: &Arc<AtomicU32>,
    abandoned_count: &Arc<AtomicU32>,
    failed_count: &Arc<AtomicU32>,
    outcomes: &Arc<Mutex<Vec<FileOutcome>>>,
) {
    *status.lock().unwrap() = JobStatus::Cancelled;
    let summary = JobSummary {
        job_id: job_id.to_string(),
        status: JobStatus::Cancelled,
        total,
        processed: processed_count.load(Ordering::Relaxed),
        abandoned: abandoned_count.load(Ordering::Relaxed),
        failed: failed_count.load(Ordering::Relaxed),
        files: outcomes.lock().unwrap().clone(),
    };
    let _ = app.emit("job_status", JobStatusEvent {
        job_id: job_id.to_string(),
        status: JobStatus::Cancelled,
    });
    let _ = app.emit("job_cancelled", &summary);
}
