use std::collections::HashMap;
use std::future::Future;
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

#[derive(Debug, Clone, PartialEq)]
enum ProcessOutcome {
    Processed,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
enum LoopExit {
    Completed,
    Cancelled,
}

fn flush_remaining_abandoned(
    files: &[FileEntry],
    outcomes: &Arc<Mutex<Vec<FileOutcome>>>,
    abandoned_count: &Arc<AtomicU32>,
) {
    for remaining in files {
        outcomes.lock().unwrap().push(FileOutcome {
            path: remaining.path.clone(),
            status: FileOutcomeStatus::Abandoned,
            in_flight: false,
        });
        abandoned_count.fetch_add(1, Ordering::Relaxed);
    }
}

/// File-iteration state machine. Pulled out of `run_job` so the pause/cancel
/// re-queue semantics (spec AC #9, #9b) can be exercised in unit tests with a
/// fake processor — `run_job` itself remains the production wiring that talks
/// to `tauri::AppHandle`, `process_receipt`, and the on-disk receipt index.
async fn run_file_loop<P, Fut, S, A>(
    files: &[FileEntry],
    mut ctrl_rx: watch::Receiver<ControlSignal>,
    processed_count: Arc<AtomicU32>,
    abandoned_count: Arc<AtomicU32>,
    failed_count: Arc<AtomicU32>,
    outcomes: Arc<Mutex<Vec<FileOutcome>>>,
    mut process: P,
    mut emit_status: S,
    mut on_abandon: A,
) -> LoopExit
where
    P: FnMut(usize, FileEntry) -> Fut,
    Fut: Future<Output = ProcessOutcome>,
    S: FnMut(JobStatus),
    A: FnMut(&str),
{
    // Mark initial value as seen so ctrl_rx.changed() waits for a real change.
    drop(ctrl_rx.borrow_and_update());

    let mut idx = 0usize;
    'file_loop: while idx < files.len() {
        let file_entry = files[idx].clone();
        // --- pause/cancel checkpoint ---
        loop {
            let sig = ctrl_rx.borrow_and_update().clone();
            match sig {
                ControlSignal::Cancel => {
                    flush_remaining_abandoned(&files[idx..], &outcomes, &abandoned_count);
                    return LoopExit::Cancelled;
                }
                ControlSignal::Pause => {
                    emit_status(JobStatus::Paused);
                    if ctrl_rx.changed().await.is_err() {
                        return LoopExit::Cancelled;
                    }
                    let next = ctrl_rx.borrow_and_update().clone();
                    if next == ControlSignal::Cancel {
                        flush_remaining_abandoned(&files[idx..], &outcomes, &abandoned_count);
                        return LoopExit::Cancelled;
                    }
                    emit_status(JobStatus::Running);
                }
                ControlSignal::Run => break,
            }
        }

        let path = file_entry.path.clone();

        tokio::select! {
            outcome = process(idx, file_entry) => {
                match outcome {
                    ProcessOutcome::Processed => {
                        processed_count.fetch_add(1, Ordering::Relaxed);
                        outcomes.lock().unwrap().push(FileOutcome {
                            path: path.clone(),
                            status: FileOutcomeStatus::Processed,
                            in_flight: false,
                        });
                    }
                    ProcessOutcome::Failed => {
                        failed_count.fetch_add(1, Ordering::Relaxed);
                        outcomes.lock().unwrap().push(FileOutcome {
                            path: path.clone(),
                            status: FileOutcomeStatus::Failed,
                            in_flight: false,
                        });
                    }
                }
                idx += 1;
            }
            _ = ctrl_rx.changed() => {
                // In-flight file dropped here (cancel-on-drop semantics for reqwest).
                let sig = ctrl_rx.borrow_and_update().clone();
                if sig == ControlSignal::Cancel {
                    // Cancel mid-file: record this file as abandoned-in-flight,
                    // then drain remaining files as unstarted abandoned.
                    on_abandon(&path);
                    abandoned_count.fetch_add(1, Ordering::Relaxed);
                    outcomes.lock().unwrap().push(FileOutcome {
                        path: path.clone(),
                        status: FileOutcomeStatus::Abandoned,
                        in_flight: true,
                    });
                    flush_remaining_abandoned(&files[idx + 1..], &outcomes, &abandoned_count);
                    return LoopExit::Cancelled;
                }
                // Pause (or Pause→Run race): re-queue the same file from
                // scratch on resume. Spec AC #9, #9b: the file index must NOT
                // advance, and no abandoned outcome is recorded since the
                // file will be retried.
                continue 'file_loop;
            }
        }
    }
    LoopExit::Completed
}

async fn run_job(
    app: AppHandle,
    job_id: String,
    config: JobConfig,
    status: Arc<Mutex<JobStatus>>,
    ctrl_rx: watch::Receiver<ControlSignal>,
    processed_count: Arc<AtomicU32>,
    abandoned_count: Arc<AtomicU32>,
    failed_count: Arc<AtomicU32>,
    outcomes: Arc<Mutex<Vec<FileOutcome>>>,
) {
    use crate::logger::{emit_log, LogLevel, Phase, ProgressEvent};
    use crate::receipt_index::{load_index, save_index, ProcessingStatus, ReceiptEntry};
    use crate::scan::ReceiptFile;

    let total = config.files.len() as u32;
    let elapsed_sum: Arc<Mutex<f64>> = Arc::new(Mutex::new(0.0));
    let config = Arc::new(config);
    let files = config.files.clone();

    let _ = app.emit("job_status", JobStatusEvent {
        job_id: job_id.clone(),
        status: JobStatus::Running,
    });

    let process = {
        let app = app.clone();
        let config = Arc::clone(&config);
        let processed_count = Arc::clone(&processed_count);
        let elapsed_sum = Arc::clone(&elapsed_sum);
        let job_id = job_id.clone();
        move |_idx: usize, file: FileEntry| {
            let app = app.clone();
            let config = Arc::clone(&config);
            let processed_count = Arc::clone(&processed_count);
            let elapsed_sum = Arc::clone(&elapsed_sum);
            let job_id = job_id.clone();
            async move {
                let path = file.path.clone();
                let receipt_file = match file.file_type.as_str() {
                    "pdf" => ReceiptFile::Pdf(PathBuf::from(&path)),
                    _ => ReceiptFile::Image(PathBuf::from(&path)),
                };

                let done_so_far = processed_count.load(Ordering::Relaxed);
                let avg_so_far = {
                    let s = *elapsed_sum.lock().unwrap();
                    if done_so_far > 0 { s / done_so_far as f64 } else { 0.0 }
                };

                let _ = app.emit("progress", ProgressEvent {
                    done: done_so_far,
                    total,
                    avg_ms: avg_so_far,
                    job_id: Some(job_id.clone()),
                    status: Some("running".to_string()),
                    current_file: Some(path.clone()),
                    phase: Some(Phase::Ocr),
                });

                emit_log(&app, LogLevel::Info, format!("Processing {}", path));
                let start = std::time::Instant::now();

                match crate::receipt::process_receipt(
                    receipt_file,
                    &config.api_url,
                    &config.api_key,
                    &config.ocr_model,
                    &config.extraction_url,
                    &config.extraction_api_key,
                    &config.extraction_model,
                    &config.active_system_prompt,
                    &config.json_schema_keys,
                ).await {
                    Ok(record) => {
                        let elapsed_ms = start.elapsed().as_millis() as f64;
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

                        let new_done = done_so_far + 1;
                        let new_sum = {
                            let mut s = elapsed_sum.lock().unwrap();
                            *s += elapsed_ms;
                            *s
                        };
                        let avg_ms = new_sum / new_done as f64;

                        emit_log(&app, LogLevel::Success, format!("Processed {}", path));
                        let _ = app.emit("progress", ProgressEvent {
                            done: new_done,
                            total,
                            avg_ms,
                            job_id: Some(job_id.clone()),
                            status: Some("running".to_string()),
                            current_file: Some(path.clone()),
                            phase: Some(Phase::Extract),
                        });
                        ProcessOutcome::Processed
                    }
                    Err(e) => {
                        emit_log(&app, LogLevel::Error, format!("Failed to process {}: {}", path, e));
                        ProcessOutcome::Failed
                    }
                }
            }
        }
    };

    let emit_status = {
        let app = app.clone();
        let job_id = job_id.clone();
        move |new_status: JobStatus| {
            let _ = app.emit("job_status", JobStatusEvent {
                job_id: job_id.clone(),
                status: new_status,
            });
        }
    };

    let on_abandon = {
        let app = app.clone();
        move |path: &str| {
            emit_log(&app, LogLevel::Warn, format!("Abandoned {} — not persisted to index", path));
        }
    };

    let exit = run_file_loop(
        &files,
        ctrl_rx,
        Arc::clone(&processed_count),
        Arc::clone(&abandoned_count),
        Arc::clone(&failed_count),
        Arc::clone(&outcomes),
        process,
        emit_status,
        on_abandon,
    ).await;

    if exit == LoopExit::Cancelled {
        emit_terminal_cancelled(&app, &job_id, &status, total, &processed_count, &abandoned_count, &failed_count, &outcomes);
        return;
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

    let elapsed_total = *elapsed_sum.lock().unwrap();
    let avg_ms = if processed > 0 { elapsed_total / processed as f64 } else { 0.0 };
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
        phase: None,
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


#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::sync::Notify;

    fn fe(path: &str) -> FileEntry {
        FileEntry { path: path.to_string(), file_type: "image".into() }
    }

    /// Spec AC #9 / #9b: pausing during in-flight processing must NOT advance
    /// the file index. On resume, the same file is re-processed from scratch.
    #[tokio::test]
    async fn pause_then_resume_reprocesses_inflight_file() {
        let files = vec![fe("A"), fe("B")];
        let (ctrl_tx, ctrl_rx) = watch::channel(ControlSignal::Run);

        let processed = Arc::new(AtomicU32::new(0));
        let abandoned = Arc::new(AtomicU32::new(0));
        let failed = Arc::new(AtomicU32::new(0));
        let outcomes: Arc<Mutex<Vec<FileOutcome>>> = Arc::new(Mutex::new(Vec::new()));

        // Records every (path) entry into the processor. With the bug, A is
        // attempted only once before being skipped; with the fix, A is
        // attempted twice (once dropped on pause, once on resume).
        let attempts: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        // Notifies when the first attempt at A is in flight, so the test can
        // send Pause at a deterministic moment.
        let inflight = Arc::new(Notify::new());

        let process = {
            let attempts = Arc::clone(&attempts);
            let inflight = Arc::clone(&inflight);
            move |_idx: usize, file: FileEntry| {
                let attempts = Arc::clone(&attempts);
                let inflight = Arc::clone(&inflight);
                async move {
                    let path = file.path.clone();
                    let attempt_n = {
                        let mut a = attempts.lock().unwrap();
                        a.push(path.clone());
                        a.iter().filter(|p| p.as_str() == path.as_str()).count()
                    };
                    if path == "A" && attempt_n == 1 {
                        // First attempt at A: signal then hang so the pause
                        // path drops us via cancel-on-drop semantics.
                        inflight.notify_one();
                        std::future::pending::<()>().await;
                        unreachable!()
                    }
                    ProcessOutcome::Processed
                }
            }
        };
        let paused_emits = Arc::new(AtomicU32::new(0));
        let emit_status = {
            let paused_emits = Arc::clone(&paused_emits);
            move |s: JobStatus| {
                if s == JobStatus::Paused {
                    paused_emits.fetch_add(1, Ordering::Relaxed);
                }
            }
        };
        let on_abandon = |_p: &str| {};

        let loop_fut = run_file_loop(
            &files,
            ctrl_rx,
            Arc::clone(&processed),
            Arc::clone(&abandoned),
            Arc::clone(&failed),
            Arc::clone(&outcomes),
            process,
            emit_status,
            on_abandon,
        );
        tokio::pin!(loop_fut);

        // Drive the loop until A enters the processor.
        tokio::select! {
            _ = inflight.notified() => {}
            _ = &mut loop_fut => panic!("loop ended before A entered processing"),
        }

        // Pause while A is in flight; drive the loop until it reaches the
        // pause checkpoint (signalled by emit_status(Paused)) and is awaiting
        // Run.
        ctrl_tx.send(ControlSignal::Pause).unwrap();
        loop {
            if paused_emits.load(Ordering::Relaxed) > 0 { break; }
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(5)) => {}
                _ = &mut loop_fut => panic!("loop ended during pause"),
            }
        }

        // Resume; the loop should re-process A from scratch, then process B.
        ctrl_tx.send(ControlSignal::Run).unwrap();
        let exit = loop_fut.await;
        assert_eq!(exit, LoopExit::Completed);

        let attempts = attempts.lock().unwrap().clone();
        let a_count = attempts.iter().filter(|p| p.as_str() == "A").count();
        assert!(
            a_count >= 2,
            "expected A to be re-attempted after resume; attempts = {:?}",
            attempts
        );
        assert!(
            attempts.contains(&"B".to_string()),
            "expected B to be processed; attempts = {:?}",
            attempts
        );
        assert_eq!(processed.load(Ordering::Relaxed), 2);
    }
}
