use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};


use crate::history;
use crate::ocr_engine::runtime::RuntimeResources;
use crate::queue::{SharedQueue, QueueStore, default_concurrency, now_millis};
use crate::security;
use crate::{
    CommandError, EnqueuePayload, FileEncryptionInfo, FileMetadata, HistoryEntry, Job, JobStatus,
    OcrOptions, QueueState, SharedHistory,
};

macro_rules! lock_or_err {
    ($lock:expr, $target:literal) => {
        match $lock {
            Ok(guard) => guard,
            Err(e) => {
                tracing::error!(target: $target, "lock poisoned: {e}");
                return Err(CommandError::queue(concat!($target, " lock poisoned")));
            }
        }
    };
    ($lock:expr, $target:literal, history) => {
        match $lock {
            Ok(guard) => guard,
            Err(e) => {
                tracing::error!(target: $target, "lock poisoned: {e}");
                return Err(CommandError::history(concat!($target, " lock poisoned")));
            }
        }
    };
    ($lock:expr, $target:literal, return) => {
        match $lock {
            Ok(guard) => guard,
            Err(e) => {
                tracing::error!(target: $target, "lock poisoned: {e}");
                return;
            }
        }
    };
}

fn global_runtime() -> &'static Arc<RuntimeResources> {
    crate::RUNTIME.get_or_init(|| Arc::new(RuntimeResources::new()))
}

/// Returns the writable tessdata directory inside the app's local data directory.
/// This is where `ensure_language_packs` downloads missing language data.
fn app_tessdata_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_local_data_dir()
        .ok()
        .map(|d| d.join("tessdata"))
        .unwrap_or_else(|| PathBuf::from("tessdata"))
}

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
        Some(app_tessdata_dir(app)),
        app.path()
            .resource_dir()
            .ok()
            .map(|dir| dir.join("tessdata")),
        crate::resolve_tessdata_path().map(PathBuf::from),
    ]
    .into_iter()
    .flatten()
    .find(|p| p.exists())
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguagePackResult {
    pub downloaded: Vec<String>,
    pub skipped: Vec<String>,
    pub errors: HashMap<String, String>,
}

#[cfg(feature = "ocr")]
#[tauri::command]
pub fn ensure_language_packs(
    app: AppHandle,
    languages: Vec<String>,
) -> Result<LanguagePackResult, CommandError> {
    if languages.is_empty() {
        return Ok(LanguagePackResult {
            downloaded: vec![],
            skipped: vec![],
            errors: HashMap::new(),
        });
    }

    let data_dir = app_tessdata_dir(&app);
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| CommandError::io(format!("Failed to create tessdata dir: {e}")))?;

    let mut result = LanguagePackResult {
        downloaded: vec![],
        skipped: vec![],
        errors: HashMap::new(),
    };

    for lang in &languages {
        let trained_path = data_dir.join(format!("{lang}.traineddata"));
        if trained_path.exists() {
            tracing::debug!(target: "knox::languages", lang, "already exists, skipping");
            result.skipped.push(lang.clone());
            continue;
        }

        // Try fast variant first, fall back to standard repo
        let urls = [
            format!("https://github.com/tesseract-ocr/tessdata_fast/raw/main/{lang}.traineddata"),
            format!("https://github.com/tesseract-ocr/tessdata/raw/main/{lang}.traineddata"),
        ];

        let mut downloaded_ok = false;
        for url in &urls {
            tracing::info!(target: "knox::languages", lang, url, "downloading");
            match reqwest::blocking::get(url) {
                Ok(response) => {
                    let status = response.status();
                    let body = match response.bytes() {
                        Ok(b) => b.to_vec(),
                        Err(e) => {
                            tracing::warn!(
                                target: "knox::languages",
                                lang,
                                error = %e,
                                "failed to read response body"
                            );
                            continue;
                        }
                    };
                    if !status.is_success() || body.len() < 100 {
                        tracing::warn!(
                            target: "knox::languages",
                            lang,
                            status = %status,
                            size = body.len(),
                            "invalid traineddata response"
                        );
                        continue;
                    }
                    if let Err(e) = std::fs::write(&trained_path, &body) {
                        tracing::warn!(
                            target: "knox::languages",
                            lang,
                            error = %e,
                            "failed to write traineddata file"
                        );
                        continue;
                    }
                    tracing::info!(
                        target: "knox::languages",
                        lang,
                        size = body.len(),
                        "downloaded successfully"
                    );
                    downloaded_ok = true;
                    break;
                }
                Err(e) => {
                    tracing::warn!(
                        target: "knox::languages",
                        lang,
                        url,
                        error = %e,
                        "download failed, trying next source"
                    );
                }
            }
        }

        if downloaded_ok {
            result.downloaded.push(lang.clone());
        } else {
            result
                .errors
                .insert(lang.clone(), "All download sources failed".to_string());
        }
    }

    Ok(result)
}

#[cfg(not(feature = "ocr"))]
#[tauri::command]
pub fn ensure_language_packs(
    _app: AppHandle,
    _languages: Vec<String>,
) -> Result<LanguagePackResult, CommandError> {
    tracing::warn!(target: "knox::languages", "OCR feature not enabled — language packs unavailable");
    Ok(LanguagePackResult {
        downloaded: vec![],
        skipped: vec![],
        errors: HashMap::new(),
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
    if let Some(parent) = p.parent()
        && !parent.is_dir()
    {
        return Err(CommandError::validation("Parent directory does not exist"));
    }
    std::fs::write(&path, &content).map_err(|e| CommandError::io(e.to_string()))
}

#[tauri::command]
pub fn get_file_metadata(path: String) -> Result<FileMetadata, CommandError> {
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
    let metadata = std::fs::metadata(&path).map_err(|e| CommandError::io(e.to_string()))?;
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

    let mut queue = lock_or_err!(state.lock(), "knox::queue");
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
    let mut queue = lock_or_err!(state.lock(), "knox::queue");
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
    let queue = lock_or_err!(state.lock(), "knox::queue");
    Ok(QueueState {
        jobs: queue.jobs.clone(),
        is_running: queue.is_running,
    })
}

#[tauri::command]
pub fn clear_queue(state: tauri::State<'_, SharedQueue>) -> Result<QueueState, CommandError> {
    let mut queue = lock_or_err!(state.lock(), "knox::queue");
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
    let history = lock_or_err!(state.lock(), "knox::history", history);
    Ok(history.entries.clone())
}

#[tauri::command]
pub fn clear_history(
    app: AppHandle,
    state: tauri::State<'_, SharedHistory>,
) -> Result<(), CommandError> {
    let mut history = lock_or_err!(state.lock(), "knox::history", history);
    history.entries.clear();
    history::save_history(&app, &history).map_err(CommandError::history)?;
    if let Err(e) = app.emit("historyUpdated", history.entries.clone()) {
        tracing::error!(target: "knox::history", "emit failed: {e}");
    }
    Ok(())
}

fn emit_queue_state(app: &AppHandle, queue: &QueueStore) {
    let snapshot = QueueState {
        jobs: queue.jobs.clone(),
        is_running: queue.is_running,
    };
    if let Err(e) = app.emit("queueState", snapshot) {
        tracing::error!(target: "knox::queue", "emit queueState failed: {e}");
    }
}

struct JobSpawn {
    index: usize,
    options: OcrOptions,
    processing: Option<crate::ocr_engine::types::ProcessingConfigInput>,
}

fn dequeue_job(state: &SharedQueue) -> Option<JobSpawn> {
    let mut queue = match state.lock() {
        Ok(lock) => lock,
        Err(e) => {
            tracing::error!(target: "knox::queue", "lock poisoned: {e}");
            return None;
        }
    };
    let index = *queue.queue.front()?;
    let options = queue.jobs[index].options.clone();
    let max_concurrency = options
        .max_concurrency
        .map(|v| v as usize)
        .unwrap_or_else(default_concurrency);
    let should_wait = options.safe_mode && queue.in_flight > 0;
    if queue.in_flight < max_concurrency && !should_wait {
        queue.queue.pop_front();
        queue.in_flight += 1;
        let processing = queue.jobs[index].processing.clone();
        Some(JobSpawn { index, options, processing })
    } else {
        None
    }
}

async fn run_job(
    app: AppHandle,
    state: SharedQueue,
    history_state: SharedHistory,
    index: usize,
    options: OcrOptions,
    processing: Option<crate::ocr_engine::types::ProcessingConfigInput>,
    job_cancelled: Arc<std::sync::atomic::AtomicBool>,
) {
    let (job_id, input_path, output_path, started_at) = {
        let mut queue = lock_or_err!(state.lock(), "knox::queue", return);
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
    tracing::info!(target: "knox::queue", job_id, input_path, "job started");

    let engine_config = match sanitize_processing_config(&app, &options, processing) {
        Ok(cfg) => cfg,
        Err(msg) => {
            tracing::error!(target: "knox::queue", job_id, "config error: {msg}");
            let mut queue = lock_or_err!(state.lock(), "knox::queue", return);
            let job = &mut queue.jobs[index];
            job.status = JobStatus::Failed;
            job.error_message = Some(msg.to_string());
            if let Err(e) = app.emit("jobProgress", job.clone()) {
                tracing::error!(target: "knox::queue", "emit jobProgress failed: {e}");
            }
            if let Err(e) = app.emit("jobFinished", job.clone()) {
                tracing::error!(target: "knox::queue", "emit jobFinished failed: {e}");
            }
            queue.in_flight = queue.in_flight.saturating_sub(1);
            return;
        }
    };
    let settings = crate::ocr_engine::types::OcrSettings::from(&options);
    let runtime = global_runtime().clone();
    let engine = crate::ocr_engine::engine::Engine::new(
        runtime,
        #[cfg(feature = "ocr")]
        app.state::<crate::ocr_engine::engine::SharedTessPool>()
            .inner()
            .clone(),
        app.state::<std::sync::Arc<crate::ocr_engine::render::PdfiumEngine>>()
            .inner()
            .clone(),
    );
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
    tracing::info!(target: "knox::queue", job_id, succeeded, was_cancelled, "job finished");

    let finished_at = now_millis();
    let duration_ms = finished_at.saturating_sub(started_at);
    let job_snapshot = {
        let mut queue = lock_or_err!(state.lock(), "knox::queue", return);
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
        history::push_history(
            &app,
            &history_state,
            HistoryEntry {
                id: job_id,
                input_path,
                output_path,
                status: if succeeded { JobStatus::Completed } else { JobStatus::Failed },
                started_at,
                finished_at,
                duration_ms,
                options: options.without_password(),
            },
        );
    }
}

#[tauri::command]
pub fn start_queue(
    app: AppHandle,
    state: tauri::State<'_, SharedQueue>,
    history: tauri::State<'_, SharedHistory>,
) -> Result<(), CommandError> {
    let mut queue = lock_or_err!(state.lock(), "knox::queue");
    if queue.is_running {
        tracing::warn!(target: "knox::queue", "start_queue called but already running");
        return Ok(());
    }
    let cancelled = queue.cancelled.clone();
    queue.cancelled.store(false, std::sync::atomic::Ordering::SeqCst);
    queue.is_running = true;
    drop(queue);

    let state = state.inner().clone();
    let history_state = history.inner().clone();
    tracing::info!(target: "knox::queue", "queue processing loop started");
    tauri::async_runtime::spawn(async move {
        loop {
            if cancelled.load(std::sync::atomic::Ordering::SeqCst) {
                let should_stop = {
                    let queue = lock_or_err!(state.lock(), "knox::queue", return);
                    if queue.in_flight == 0 {
                        emit_queue_state(&app, &*queue);
                        true
                    } else {
                        false
                    }
                };
                if should_stop {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(120)).await;
                continue;
            }

            if let Some(spawn) = dequeue_job(&state) {
                let app = app.clone();
                let state = state.clone();
                let h = history_state.clone();
                let cc = cancelled.clone();
                tauri::async_runtime::spawn(run_job(
                    app, state, h, spawn.index, spawn.options, spawn.processing, cc,
                ));
            } else {
                let should_stop = {
                    let mut queue = lock_or_err!(state.lock(), "knox::queue", return);
                    if queue.in_flight == 0 && queue.queue.is_empty() {
                        queue.is_running = false;
                        emit_queue_state(&app, &*queue);
                        true
                    } else {
                        false
                    }
                };
                if should_stop {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(120)).await;
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
    let mut queue = lock_or_err!(state.lock(), "knox::queue");
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

#[tauri::command]
pub fn check_file_encrypted(path: String) -> Result<FileEncryptionInfo, CommandError> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(CommandError::validation(format!(
            "File does not exist: {path}"
        )));
    }
    let canonical = p
        .canonicalize()
        .map_err(|e| CommandError::io(format!("Failed to canonicalize path: {e}")))?;
    let file_id = canonical.to_string_lossy().to_string();

    match lopdf::Document::load(p) {
        Ok(doc) => Ok(FileEncryptionInfo {
            encrypted: doc.is_encrypted(),
            file_id,
        }),
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            if msg.contains("encrypt") || msg.contains("password") {
                Ok(FileEncryptionInfo {
                    encrypted: true,
                    file_id,
                })
            } else {
                Err(CommandError::pipeline(format!(
                    "Failed to load PDF: {e}"
                )))
            }
        }
    }
}

#[tauri::command]
pub fn log_window_shown() {
    let elapsed = crate::START_TIME
        .get()
        .map(|t| t.elapsed())
        .unwrap_or_default();
    tracing::info!(
        target: "knox::startup",
        elapsed_ms = elapsed.as_millis() as u64,
        "window shown — app ready"
    );
}
