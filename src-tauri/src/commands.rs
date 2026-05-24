use std::path::{Path, PathBuf};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tokio::time::sleep;

use crate::history;
use crate::queue::{SharedQueue, default_concurrency, now_millis};
use crate::security;
use crate::{
    CommandError, EnqueuePayload, FileMetadata, HistoryEntry, Job, JobStatus, OcrOptions,
    OutputType, QueueState, SharedHistory,
};

fn sanitize_processing_config(
    app: &AppHandle,
    _input: &OcrOptions,
    processing: Option<crate::ocr_engine::types::ProcessingConfigInput>,
) -> Result<crate::ocr_engine::types::ProcessingConfig, CommandError> {
    let tessdata_path: PathBuf = [
        processing
            .as_ref()
            .and_then(|cfg| cfg.tessdata_path.as_ref())
            .filter(|v| !v.is_empty())
            .map(PathBuf::from),
        app.path()
            .resource_dir()
            .ok()
            .map(|dir| dir.join("tessdata")),
        std::env::var("TESSDATA_PREFIX").ok().map(PathBuf::from),
        Some(PathBuf::from("/opt/homebrew/share/tessdata")),
        Some(PathBuf::from("/usr/local/share/tessdata/")),
        Some(PathBuf::from("/usr/share/tessdata/")),
    ]
    .into_iter()
    .flatten()
    .filter(|p| p.exists())
    .next()
    .ok_or_else(|| CommandError::pipeline("Unable to resolve tessdata path"))?;

    let max_concurrent_files = processing
        .as_ref()
        .and_then(|cfg| cfg.max_concurrent_files)
        .or(_input.memory_pages);
    let thread_pool_size = processing.as_ref().and_then(|cfg| cfg.thread_pool_size);
    let languages = processing
        .as_ref()
        .and_then(|cfg| cfg.languages.as_ref())
        .filter(|value| !value.is_empty())
        .cloned()
        .or_else(|| _input.languages.clone())
        .unwrap_or_else(|| "eng".to_string());
    Ok(crate::ocr_engine::types::ProcessingConfig {
        max_concurrent_files,
        tessdata_path: tessdata_path.to_string_lossy().to_string(),
        languages,
        thread_pool_size,
    })
}

fn validate_input_path(path: &str) -> Result<(), CommandError> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(CommandError::validation(format!(
            "Input file does not exist: {path}"
        )));
    }
    if !p.is_file() {
        return Err(CommandError::validation(format!(
            "Input is not a file: {path}"
        )));
    }
    match p.extension().and_then(|e| e.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("pdf") => Ok(()),
        _ => Err(CommandError::validation(format!(
            "Input is not a PDF file: {path}"
        ))),
    }
}

#[tauri::command]
pub fn write_log_file(path: String, content: String) -> Result<(), CommandError> {
    let p = Path::new(&path);
    if !p.is_absolute() {
        return Err(CommandError::validation("Path must be absolute"));
    }
    if p.extension().and_then(|e| e.to_str()) != Some("log") {
        return Err(CommandError::validation("File must have .log extension"));
    }
    if let Some(parent) = p.parent() {
        if !parent.is_dir() {
            return Err(CommandError::validation("Parent directory does not exist"));
        }
    }
    std::fs::write(&path, &content).map_err(|e| CommandError::io(e.to_string()))
}

#[tauri::command]
pub fn get_file_metadata(path: String) -> Result<FileMetadata, CommandError> {
    use std::fs;
    let p = Path::new(&path);
    if !p.exists() {
        return Err(CommandError::validation(format!(
            "File does not exist: {path}"
        )));
    }
    if !p.is_file() {
        return Err(CommandError::validation(format!(
            "Not a regular file: {path}"
        )));
    }
    let metadata = fs::metadata(&path).map_err(|e| CommandError::io(e.to_string()))?;
    Ok(FileMetadata {
        size: metadata.len(),
    })
}

#[tauri::command]
pub fn enqueue(
    state: tauri::State<'_, SharedQueue>,
    payload: EnqueuePayload,
) -> Result<QueueState, CommandError> {
    let output_dir = PathBuf::from(payload.output_dir);
    security::validate_output_dir(&output_dir).map_err(CommandError::validation)?;

    for file_path in &payload.files {
        validate_input_path(file_path)?;
    }

    let mut queue = state.lock().map_err(|e| {
        tracing::error!(target: "knox::queue", "lock poisoned: {e}");
        CommandError::queue("Queue lock poisoned")
    })?;
    let count = payload.files.len();
    for file in payload.files {
        let output_path = security::safe_output_path(&output_dir, Path::new(&file));
        let job = Job {
            id: uuid::Uuid::new_v4().to_string(),
            input_path: file,
            output_path: output_path.to_string_lossy().to_string(),
            status: JobStatus::Queued,
            percent: 0,
            started_at: None,
            finished_at: None,
            options: payload.options.clone(),
            processing: payload.processing.clone(),
            error_message: None,
        };
        let index = queue.jobs.len();
        queue.jobs.push(job);
        queue.queue.push_back(index);
    }
    tracing::info!(target: "knox::queue", count, "files enqueued");

    Ok(QueueState {
        jobs: queue.jobs.clone(),
        is_running: queue.is_running,
    })
}

#[tauri::command]
pub fn remove_job(
    state: tauri::State<'_, SharedQueue>,
    job_id: String,
) -> Result<QueueState, CommandError> {
    let mut queue = state.lock().map_err(|e| {
        tracing::error!(target: "knox::queue", "lock poisoned: {e}");
        CommandError::queue("Queue lock poisoned")
    })?;
    let index = queue
        .jobs
        .iter()
        .position(|job| job.id == job_id)
        .ok_or_else(|| CommandError::validation("Job not found"))?;
    if !matches!(
        queue.jobs[index].status,
        JobStatus::Queued | JobStatus::Cancelled
    ) {
        return Err(CommandError::validation("Job is already running"));
    }
    queue.jobs.remove(index);
    queue.queue = queue
        .queue
        .iter()
        .filter_map(|&idx| {
            if idx == index {
                None
            } else if idx > index {
                Some(idx - 1)
            } else {
                Some(idx)
            }
        })
        .collect();
    tracing::info!(target: "knox::queue", job_id, "job removed");
    Ok(QueueState {
        jobs: queue.jobs.clone(),
        is_running: queue.is_running,
    })
}

#[tauri::command]
pub fn get_status(state: tauri::State<'_, SharedQueue>) -> Result<QueueState, CommandError> {
    let queue = state.lock().map_err(|e| {
        tracing::error!(target: "knox::queue", "lock poisoned: {e}");
        CommandError::queue("Queue lock poisoned")
    })?;
    Ok(QueueState {
        jobs: queue.jobs.clone(),
        is_running: queue.is_running,
    })
}

#[tauri::command]
pub fn clear_queue(state: tauri::State<'_, SharedQueue>) -> Result<QueueState, CommandError> {
    let mut queue = state.lock().map_err(|e| {
        tracing::error!(target: "knox::queue", "lock poisoned: {e}");
        CommandError::queue("Queue lock poisoned")
    })?;
    let count = queue.jobs.len();
    queue.jobs.clear();
    queue.queue.clear();
    queue.in_flight = 0;
    tracing::info!(target: "knox::queue", count, "queue cleared");
    Ok(QueueState {
        jobs: Vec::new(),
        is_running: queue.is_running,
    })
}

#[tauri::command]
pub fn get_history(
    state: tauri::State<'_, SharedHistory>,
) -> Result<Vec<HistoryEntry>, CommandError> {
    let history = state.lock().map_err(|e| {
        tracing::error!(target: "knox::history", "lock poisoned: {e}");
        CommandError::history("History lock poisoned")
    })?;
    Ok(history.entries.clone())
}

#[tauri::command]
pub fn clear_history(
    app: AppHandle,
    state: tauri::State<'_, SharedHistory>,
) -> Result<(), CommandError> {
    let mut history = state.lock().map_err(|e| {
        tracing::error!(target: "knox::history", "lock poisoned: {e}");
        CommandError::history("History lock poisoned")
    })?;
    history.entries.clear();
    history::save_history(&app, &history).map_err(CommandError::history)?;
    if let Err(e) = app.emit("historyUpdated", history.entries.clone()) {
        tracing::error!(target: "knox::history", "emit failed: {e}");
    }
    Ok(())
}

#[tauri::command]
pub fn start_queue(
    app: AppHandle,
    state: tauri::State<'_, SharedQueue>,
    history: tauri::State<'_, SharedHistory>,
) -> Result<(), CommandError> {
    let mut queue = state.lock().map_err(|e| {
        tracing::error!(target: "knox::queue", "lock poisoned: {e}");
        CommandError::queue("Queue lock poisoned")
    })?;
    if queue.is_running {
        tracing::warn!(target: "knox::queue", "start_queue called but already running");
        return Ok(());
    }
    let cancelled = queue.cancelled.clone();
    queue
        .cancelled
        .store(false, std::sync::atomic::Ordering::SeqCst);
    queue.is_running = true;
    drop(queue);

    let state = state.inner().clone();
    let history_state = history.inner().clone();
    tracing::info!(target: "knox::queue", "queue processing loop started");
    tauri::async_runtime::spawn(async move {
        loop {
            let (next_index, job_options, pause) = {
                let mut queue = match state.lock() {
                    Ok(lock) => lock,
                    Err(e) => {
                        tracing::error!(target: "knox::queue", "lock poisoned: {e}");
                        return;
                    }
                };

                if !queue.is_running {
                    if queue.in_flight == 0 {
                        let snapshot = QueueState {
                            jobs: queue.jobs.clone(),
                            is_running: queue.is_running,
                        };
                        if let Err(e) = app.emit("queueState", snapshot) {
                            tracing::error!(target: "knox::queue", "emit queueState failed: {e}");
                        }
                        return;
                    }
                    (None, None, true)
                } else if let Some(&index) = queue.queue.front() {
                    let options = queue.jobs[index].options.clone();
                    let max_concurrency = options
                        .max_concurrency
                        .map(|value| value as usize)
                        .unwrap_or_else(default_concurrency);
                    let should_wait = options.safe_mode && queue.in_flight > 0;
                    let can_start = queue.in_flight < max_concurrency && !should_wait;
                    if can_start {
                        queue.queue.pop_front();
                        queue.in_flight += 1;
                        (Some(index), Some(options), false)
                    } else {
                        (None, None, true)
                    }
                } else if queue.in_flight == 0 {
                    queue.is_running = false;
                    let snapshot = QueueState {
                        jobs: queue.jobs.clone(),
                        is_running: queue.is_running,
                    };
                    if let Err(e) = app.emit("queueState", snapshot) {
                        tracing::error!(target: "knox::queue", "emit queueState failed: {e}");
                    }
                    return;
                } else {
                    (None, None, true)
                }
            };

            if let Some(index) = next_index {
                let app = app.clone();
                let state = state.clone();
                let history_state = history_state.clone();
                let job_cancelled = cancelled.clone();
                let processing = {
                    let queue = state.lock().ok();
                    queue.and_then(|queue| {
                        queue.jobs.get(index).and_then(|job| job.processing.clone())
                    })
                };
                let options = job_options.unwrap_or_else(|| OcrOptions {
                    output_type: OutputType::Pdfa,
                    lossy_compression: true,
                    jpeg_quality: 60,
                    deskew: false,
                    clean: false,
                    remove_background: false,
                    preserve_metadata: true,
                    safe_mode: false,
                    max_concurrency: None,
                    per_job_threads: None,
                    binarization: crate::ocr_engine::types::BinarizationMode::Otsu,
                    fixed_threshold: 128,
                    deskew_mode: crate::ocr_engine::types::DeskewMode::Radon,
                    denoise_level: 2,
                    existing_text: crate::ocr_engine::types::ExistingTextMode::Skip,
                    psm: crate::ocr_engine::types::PageSegMode::Auto,
                    compression: crate::ocr_engine::types::CompressionMode::Ccitt,
                    resolution_dpi: 300,
                    archive_enforcement: false,
                    languages: None,
                    memory_pages: None,
                });
                tauri::async_runtime::spawn(async move {
                    let (job_id, input_path, output_path, started_at) = {
                        let mut queue = match state.lock() {
                            Ok(lock) => lock,
                            Err(e) => {
                                tracing::error!(target: "knox::queue", "lock poisoned: {e}");
                                return;
                            }
                        };
                        let job = &mut queue.jobs[index];
                        job.status = JobStatus::Running;
                        job.percent = 0;
                        job.started_at = Some(now_millis());
                        if let Err(e) = app.emit("jobProgress", job.clone()) {
                            tracing::error!(target: "knox::queue", "emit jobProgress failed: {e}");
                        }
                        (
                            job.id.clone(),
                            job.input_path.clone(),
                            job.output_path.clone(),
                            job.started_at.unwrap_or_else(now_millis),
                        )
                    };
                    tracing::info!(
                        target: "knox::queue",
                        job_id,
                        input_path,
                        "job started"
                    );

                    let engine_config = match sanitize_processing_config(&app, &options, processing)
                    {
                        Ok(cfg) => cfg,
                        Err(msg) => {
                            tracing::error!(
                                target: "knox::queue",
                                job_id,
                                "config error: {msg}"
                            );
                            let mut queue = match state.lock() {
                                Ok(lock) => lock,
                                Err(e) => {
                                    tracing::error!(
                                        target: "knox::queue",
                                        "lock poisoned: {e}"
                                    );
                                    return;
                                }
                            };
                            let job = &mut queue.jobs[index];
                            job.status = JobStatus::Failed;
                            job.error_message = Some(msg.to_string());
                            if let Err(e) = app.emit("jobProgress", job.clone()) {
                                tracing::error!(
                                    target: "knox::queue",
                                    "emit jobProgress failed: {e}"
                                );
                            }
                            if let Err(e) = app.emit("jobFinished", job.clone()) {
                                tracing::error!(
                                    target: "knox::queue",
                                    "emit jobFinished failed: {e}"
                                );
                            }
                            queue.in_flight = queue.in_flight.saturating_sub(1);
                            return;
                        }
                    };
                    let settings = crate::ocr_engine::types::OcrSettings::from(&options);
                    let engine = crate::ocr_engine::engine::Engine::new(&engine_config, &settings);
                    let result = engine
                        .process_files(
                            app.clone(),
                            engine_config,
                            settings,
                            vec![crate::ocr_engine::ingest::IngestItem {
                                job_id: job_id.clone(),
                                path: PathBuf::from(&input_path),
                                output_path: PathBuf::from(&output_path),
                            }],
                            4,
                            job_cancelled,
                        )
                        .await;
                    let was_cancelled = matches!(
                        &result,
                        Err(crate::ocr_engine::error::PipelineError::Cancelled)
                    );
                    let succeeded = result.is_ok();
                    tracing::info!(
                        target: "knox::queue",
                        job_id,
                        succeeded,
                        was_cancelled,
                        "job finished"
                    );

                    let finished_at = now_millis();
                    let duration_ms = finished_at.saturating_sub(started_at);
                    let job_snapshot = {
                        let mut queue = match state.lock() {
                            Ok(lock) => lock,
                            Err(e) => {
                                tracing::error!(
                                    target: "knox::queue",
                                    "lock poisoned: {e}"
                                );
                                return;
                            }
                        };
                        queue.in_flight = queue.in_flight.saturating_sub(1);
                        let job = &mut queue.jobs[index];
                        job.status = if was_cancelled {
                            JobStatus::Cancelled
                        } else if succeeded {
                            JobStatus::Completed
                        } else {
                            JobStatus::Failed
                        };
                        job.percent = if succeeded { 100 } else { job.percent };
                        job.finished_at = Some(finished_at);
                        if was_cancelled {
                            job.error_message = None;
                        } else if let Err(ref err) = result {
                            job.error_message = Some(err.to_string());
                        }
                        job.clone()
                    };
                    if let Err(e) = app.emit("jobFinished", job_snapshot) {
                        tracing::error!(target: "knox::queue", "emit jobFinished failed: {e}");
                    }

                    if !was_cancelled {
                        let history_entry = HistoryEntry {
                            id: job_id,
                            input_path,
                            output_path,
                            status: if succeeded {
                                JobStatus::Completed
                            } else {
                                JobStatus::Failed
                            },
                            started_at,
                            finished_at,
                            duration_ms,
                            options,
                        };
                        history::push_history(&app, &history_state, history_entry);
                    }
                });
            }

            if pause {
                sleep(Duration::from_millis(120)).await;
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn pause_queue(
    app: AppHandle,
    state: tauri::State<'_, SharedQueue>,
) -> Result<(), CommandError> {
    let mut queue = state.lock().map_err(|e| {
        tracing::error!(target: "knox::queue", "lock poisoned: {e}");
        CommandError::queue("Queue lock poisoned")
    })?;
    queue.is_running = false;
    queue
        .cancelled
        .store(true, std::sync::atomic::Ordering::SeqCst);
    tracing::info!(target: "knox::queue", "queue paused");
    let snapshot = QueueState {
        jobs: queue.jobs.clone(),
        is_running: queue.is_running,
    };
    if let Err(e) = app.emit("queueState", snapshot) {
        tracing::error!(target: "knox::queue", "emit queueState failed: {e}");
    }
    Ok(())
}
