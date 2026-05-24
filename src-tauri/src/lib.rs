use std::fmt;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::Manager;

mod commands;
mod history;
mod queue;
mod security;
mod ocr_engine;
use tauri_plugin_dialog::init as dialog_init;

/// Typed error returned by Tauri commands.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub kind: String,
    pub message: String,
}

impl CommandError {
    fn validation(msg: impl Into<String>) -> Self {
        Self { kind: "validation".into(), message: msg.into() }
    }
    fn io(msg: impl Into<String>) -> Self {
        Self { kind: "io".into(), message: msg.into() }
    }
    fn queue(msg: impl Into<String>) -> Self {
        Self { kind: "queue".into(), message: msg.into() }
    }
    fn history(msg: impl Into<String>) -> Self {
        Self { kind: "history".into(), message: msg.into() }
    }
    fn pipeline(msg: impl Into<String>) -> Self {
        Self { kind: "pipeline".into(), message: msg.into() }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

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
pub struct HistoryStore {
    pub entries: Vec<HistoryEntry>,
}

pub type SharedHistory = Arc<Mutex<HistoryStore>>;

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
        .manage(Arc::new(Mutex::new(queue::QueueStore::default())))
        .manage(Arc::new(Mutex::new(HistoryStore::default())))
        .setup(|app| {
            let history_state: tauri::State<SharedHistory> = app.state();
            let loaded = history::load_history(app.handle());
            let mut history = history_state.lock().map_err(|e| {
                tracing::error!(target: "knox::history", "setup lock poisoned: {e}");
                "History lock poisoned".to_string()
            })?;
            *history = loaded;
            tracing::info!(target: "knox::history", entries = history.entries.len(), "history loaded");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::enqueue,
            commands::get_status,
            commands::clear_queue,
            commands::remove_job,
            commands::start_queue,
            commands::pause_queue,
            commands::get_history,
            commands::clear_history,
            commands::write_log_file,
            commands::get_file_metadata
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
