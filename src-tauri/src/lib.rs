use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::sleep;
use std::fs;

mod security;
mod ocr_engine;
use tauri_plugin_dialog::init as dialog_init;

#[derive(Default)]
struct RunnerConfig {
    executable: Option<PathBuf>,
}

type SharedRunner = Arc<Mutex<RunnerConfig>>;

const HISTORY_LIMIT: usize = 100;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    pub size: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrOptions {
    pub output_type: OutputType,
    pub lossy_compression: bool,
    pub jpeg_quality: u8,
    pub deskew: bool,
    pub clean: bool,
    pub remove_background: bool,
    pub preserve_metadata: bool,
    pub safe_mode: bool,
    pub max_concurrency: Option<u8>,
    pub per_job_threads: Option<u8>,
    pub binarization: ocr_engine::types::BinarizationMode,
    pub fixed_threshold: u8,
    pub deskew_mode: ocr_engine::types::DeskewMode,
    pub denoise_level: u8,
    pub existing_text: ocr_engine::types::ExistingTextMode,
    pub psm: ocr_engine::types::PageSegMode,
    pub compression: ocr_engine::types::CompressionMode,
    pub resolution_dpi: u16,
    pub archive_enforcement: bool,
    pub languages: Option<String>,
    pub memory_pages: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerStatus {
    pub configured: bool,
    pub path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerConfigSnapshot {
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputType {
    Pdfa,
    Pdf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnqueuePayload {
    pub files: Vec<String>,
    pub output_dir: String,
    pub options: OcrOptions,
    pub processing: Option<ocr_engine::types::ProcessingConfigInput>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub input_path: String,
    pub output_path: String,
    pub status: JobStatus,
    pub percent: u8,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub options: OcrOptions,
    pub processing: Option<ocr_engine::types::ProcessingConfigInput>,
    pub error_message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueState {
    pub jobs: Vec<Job>,
    pub is_running: bool,
}

struct QueueStore {
    jobs: Vec<Job>,
    queue: VecDeque<usize>,
    is_running: bool,
    in_flight: usize,
    cancelled: Arc<AtomicBool>,
}

impl Default for QueueStore {
    fn default() -> Self {
        Self {
            jobs: Vec::new(),
            queue: VecDeque::new(),
            is_running: false,
            in_flight: 0,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
}

type SharedQueue = Arc<Mutex<QueueStore>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub input_path: String,
    pub output_path: String,
    pub status: JobStatus,
    pub started_at: u64,
    pub finished_at: u64,
    pub duration_ms: u64,
    pub options: OcrOptions,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HistoryStore {
    entries: Vec<HistoryEntry>,
}

type SharedHistory = Arc<Mutex<HistoryStore>>;

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_millis() as u64
}

fn default_concurrency() -> usize {
    let cores = num_cpus::get_physical().max(1);
    let half = (cores / 2).max(1);
    std::cmp::min(2, half)
}

fn history_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "Unable to resolve app data directory".to_string())?;
    std::fs::create_dir_all(&dir).map_err(|_| "Unable to create app data directory".to_string())?;
    Ok(dir.join("history.json"))
}

fn load_history(app: &AppHandle) -> HistoryStore {
    let path = match history_path(app) {
        Ok(path) => path,
        Err(_) => return HistoryStore::default(),
    };
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(_) => return HistoryStore::default(),
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

fn save_history(app: &AppHandle, store: &HistoryStore) -> Result<(), String> {
    let path = history_path(app)?;
    let data = serde_json::to_string_pretty(store).map_err(|_| "Unable to serialize history".to_string())?;
    std::fs::write(path, data).map_err(|_| "Unable to write history".to_string())
}

#[tauri::command]
fn write_log_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, &content).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_file_metadata(path: String) -> Result<FileMetadata, String> {
    let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
    Ok(FileMetadata {
        size: metadata.len(),
    })
}

#[tauri::command]
fn set_runner_path(state: tauri::State<SharedRunner>, path: String) -> Result<RunnerStatus, String> {
    let mut runner = state.lock().map_err(|_| "Runner lock poisoned".to_string())?;
    let resolved = PathBuf::from(path);
    if !resolved.exists() {
        return Err("Runner path does not exist".to_string());
    }
    runner.executable = Some(resolved.clone());
    Ok(RunnerStatus {
        configured: true,
        path: Some(resolved.to_string_lossy().to_string()),
    })
}

#[tauri::command]
fn get_runner_status(state: tauri::State<SharedRunner>) -> Result<RunnerStatus, String> {
    let runner = state.lock().map_err(|_| "Runner lock poisoned".to_string())?;
    Ok(RunnerStatus {
        configured: runner.executable.is_some(),
        path: runner.executable.as_ref().map(|p| p.to_string_lossy().to_string()),
    })
}

fn sanitize_processing_config(
    app: &AppHandle,
    _input: &OcrOptions,
    processing: Option<ocr_engine::types::ProcessingConfigInput>,
) -> Result<ocr_engine::types::ProcessingConfig, String> {
    let tessdata_path: PathBuf = [
        processing
            .as_ref()
            .and_then(|cfg| cfg.tessdata_path.as_ref())
            .filter(|v| !v.is_empty())
            .map(PathBuf::from),
        app.path().resource_dir().ok().map(|dir| dir.join("tessdata")),
        std::env::var("TESSDATA_PREFIX").ok().map(PathBuf::from),
        Some(PathBuf::from("/opt/homebrew/share/tessdata")),
        Some(PathBuf::from("/usr/local/share/tessdata/")),
        Some(PathBuf::from("/usr/share/tessdata/")),
    ]
    .into_iter()
    .flatten()
    .filter(|p| p.exists())
    .next()
    .ok_or_else(|| "Unable to resolve tessdata path".to_string())?;


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
    Ok(ocr_engine::types::ProcessingConfig {
        max_concurrent_files,
        tessdata_path: tessdata_path.to_string_lossy().to_string(),
        languages,
        thread_pool_size,
    })
}

#[tauri::command]
fn enqueue(state: tauri::State<SharedQueue>, payload: EnqueuePayload) -> Result<QueueState, String> {
    let output_dir = PathBuf::from(payload.output_dir);
    security::validate_output_dir(&output_dir)?;

        let mut queue = state.lock().map_err(|_| "Queue lock poisoned".to_string())?;
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

    Ok(QueueState {
        jobs: queue.jobs.clone(),
        is_running: queue.is_running,
    })
}

#[tauri::command]
fn remove_job(state: tauri::State<SharedQueue>, job_id: String) -> Result<QueueState, String> {
    let mut queue = state.lock().map_err(|_| "Queue lock poisoned".to_string())?;
    let index = queue
        .jobs
        .iter()
        .position(|job| job.id == job_id)
        .ok_or_else(|| "Job not found".to_string())?;
    if !matches!(queue.jobs[index].status, JobStatus::Queued | JobStatus::Cancelled) {
        return Err("Job is already running".to_string());
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
    Ok(QueueState {
        jobs: queue.jobs.clone(),
        is_running: queue.is_running,
    })
}

#[tauri::command]
fn get_status(state: tauri::State<SharedQueue>) -> Result<QueueState, String> {
    let queue = state.lock().map_err(|_| "Queue lock poisoned".to_string())?;
    Ok(QueueState {
        jobs: queue.jobs.clone(),
        is_running: queue.is_running,
    })
}

#[tauri::command]
fn clear_queue(state: tauri::State<SharedQueue>) -> Result<QueueState, String> {
    let mut queue = state.lock().map_err(|_| "Queue lock poisoned".to_string())?;
    queue.jobs.clear();
    queue.queue.clear();
    queue.in_flight = 0;
    Ok(QueueState {
        jobs: Vec::new(),
        is_running: queue.is_running,
    })
}

#[tauri::command]
fn get_history(state: tauri::State<SharedHistory>) -> Result<Vec<HistoryEntry>, String> {
    let history = state.lock().map_err(|_| "History lock poisoned".to_string())?;
    Ok(history.entries.clone())
}

#[tauri::command]
fn clear_history(app: AppHandle, state: tauri::State<SharedHistory>) -> Result<(), String> {
    let mut history = state.lock().map_err(|_| "History lock poisoned".to_string())?;
    history.entries.clear();
    save_history(&app, &history)?;
    let _ = app.emit("historyUpdated", history.entries.clone());
    Ok(())
}

fn push_history(app: &AppHandle, state: &SharedHistory, entry: HistoryEntry) {
    let mut history = match state.lock() {
        Ok(lock) => lock,
        Err(_) => return,
    };
    history.entries.insert(0, entry);
    if history.entries.len() > HISTORY_LIMIT {
        history.entries.truncate(HISTORY_LIMIT);
    }
    let _ = save_history(app, &history);
    let _ = app.emit("historyUpdated", history.entries.clone());
}

#[tauri::command]
fn start_queue(app: AppHandle, state: tauri::State<SharedQueue>, history: tauri::State<SharedHistory>) -> Result<(), String> {
    let mut queue = state.lock().map_err(|_| "Queue lock poisoned".to_string())?;
    if queue.is_running {
        return Ok(());
    }
    let cancelled = queue.cancelled.clone();
    queue.cancelled.store(false, Ordering::SeqCst);
    queue.is_running = true;
    drop(queue);

    let state = state.inner().clone();
    let history_state = history.inner().clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let (next_index, job_options, pause) = {
                let mut queue = match state.lock() {
                    Ok(lock) => lock,
                    Err(_) => return,
                };

                if !queue.is_running {
                    if queue.in_flight == 0 {
                        let snapshot = QueueState {
                            jobs: queue.jobs.clone(),
                            is_running: queue.is_running,
                        };
                        let _ = app.emit("queueState", snapshot);
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
                    let _ = app.emit("queueState", snapshot);
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
                    queue.and_then(|queue| queue.jobs.get(index).and_then(|job| job.processing.clone()))
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
                        binarization: ocr_engine::types::BinarizationMode::Otsu,
                        fixed_threshold: 128,
                        deskew_mode: ocr_engine::types::DeskewMode::Radon,
                        denoise_level: 2,
                        existing_text: ocr_engine::types::ExistingTextMode::Skip,
                        psm: ocr_engine::types::PageSegMode::Auto,
                        compression: ocr_engine::types::CompressionMode::Ccitt,
                        resolution_dpi: 300,
                        archive_enforcement: false,
                        languages: None,
                        memory_pages: None,
                    });
                tauri::async_runtime::spawn(async move {
                    let (job_id, input_path, output_path, started_at) = {
                        let mut queue = match state.lock() {
                            Ok(lock) => lock,
                            Err(_) => return,
                        };
                        let job = &mut queue.jobs[index];
                        job.status = JobStatus::Running;
                        job.percent = 0;
                        job.started_at = Some(now_millis());
                        let _ = app.emit("jobProgress", job.clone());
                        (
                            job.id.clone(),
                            job.input_path.clone(),
                            job.output_path.clone(),
                            job.started_at.unwrap_or_else(now_millis),
                        )
                    };

                    let engine_config = match sanitize_processing_config(&app, &options, processing) {
                        Ok(cfg) => cfg,
                        Err(msg) => {
                            let mut queue = match state.lock() {
                                Ok(lock) => lock,
                                Err(_) => return,
                            };
                            let job = &mut queue.jobs[index];
                            job.status = JobStatus::Failed;
                            job.error_message = Some(msg);
                            let _ = app.emit("jobProgress", job.clone());
                            let _ = app.emit("jobFinished", job.clone());
                            queue.in_flight = queue.in_flight.saturating_sub(1);
                            return;
                        }
                    };
                    let settings = ocr_engine::types::OcrSettings::from(&options);
                    let engine = ocr_engine::engine::Engine::new(&engine_config, &settings);
                    let result = engine
                        .process_files(
                            app.clone(),
                            engine_config,
                            settings,
                            vec![ocr_engine::ingest::IngestItem {
                                job_id: job_id.clone(),
                                path: PathBuf::from(&input_path),
                                output_path: PathBuf::from(&output_path),
                            }],
                            4,
                            job_cancelled,
                        )
                        .await;
                    let was_cancelled = matches!(&result, Err(ocr_engine::error::PipelineError::Cancelled));
                    let succeeded = result.is_ok();

                    let finished_at = now_millis();
                    let duration_ms = finished_at.saturating_sub(started_at);
                    let job_snapshot = {
                        let mut queue = match state.lock() {
                            Ok(lock) => lock,
                            Err(_) => return,
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
                    let _ = app.emit("jobFinished", job_snapshot);

                    if !was_cancelled {
                        let history_entry = HistoryEntry {
                            id: job_id,
                            input_path,
                            output_path,
                            status: if succeeded { JobStatus::Completed } else { JobStatus::Failed },
                            started_at,
                            finished_at,
                            duration_ms,
                            options,
                        };
                        push_history(&app, &history_state, history_entry);
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
fn pause_queue(app: AppHandle, state: tauri::State<SharedQueue>) -> Result<(), String> {
    let mut queue = state.lock().map_err(|_| "Queue lock poisoned".to_string())?;
    queue.is_running = false;
    queue.cancelled.store(true, Ordering::SeqCst);
    let snapshot = QueueState {
        jobs: queue.jobs.clone(),
        is_running: queue.is_running,
    };
    let _ = app.emit("queueState", snapshot);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
        .with_writer(std::io::stderr)
        .init();
    tracing::info!("application started");
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(dialog_init())
        .manage(Arc::new(Mutex::new(QueueStore::default())))
        .manage(Arc::new(Mutex::new(HistoryStore::default())))
        .manage(Arc::new(Mutex::new(RunnerConfig::default())))
        .setup(|app| {
            let history_state: tauri::State<SharedHistory> = app.state();
            let loaded = load_history(app.handle());
            let mut history = history_state.lock().map_err(|_| "History lock poisoned".to_string())?;
            *history = loaded;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            enqueue,
            get_status,
            clear_queue,
            remove_job,
            start_queue,
            pause_queue,
            get_history,
            clear_history,
            set_runner_path,
            get_runner_status,
            write_log_file,
            get_file_metadata
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
