use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::{Deserialize, Serialize};
#[cfg(feature = "typescript")]
use ts_rs::TS;
use tauri::Manager;

#[cfg(feature = "integration")]
pub mod commands;
#[cfg(not(feature = "integration"))]
mod commands;
mod history;
pub mod ocr_engine;
#[cfg(feature = "integration")]
pub mod queue;
#[cfg(not(feature = "integration"))]
mod queue;
mod security;
use ocr_engine::runtime::RuntimeResources;
use tauri_plugin_dialog::init as dialog_init;

/// Typed error returned by Tauri commands.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub kind: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct FileEncryptionInfo {
    pub encrypted: bool,
    pub file_id: String,
}

impl CommandError {
    fn validation(msg: impl Into<String>) -> Self {
        Self {
            kind: "validation".into(),
            message: msg.into(),
        }
    }
    fn io(msg: impl Into<String>) -> Self {
        Self {
            kind: "io".into(),
            message: msg.into(),
        }
    }
    fn queue(msg: impl Into<String>) -> Self {
        Self {
            kind: "queue".into(),
            message: msg.into(),
        }
    }
    fn history(msg: impl Into<String>) -> Self {
        Self {
            kind: "history".into(),
            message: msg.into(),
        }
    }
    fn pipeline(msg: impl Into<String>) -> Self {
        Self {
            kind: "pipeline".into(),
            message: msg.into(),
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    pub size: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct OcrOptions {
    pub output_type: OutputType,
    pub safe_mode: bool,
    pub max_concurrency: Option<u8>,
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
    pub continue_on_error: bool,
    pub password: Option<String>,
}

impl OcrOptions {
    pub fn without_password(&self) -> Self {
        let mut opts = self.clone();
        opts.password = None;
        opts
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "lowercase")]
pub enum OutputType {
    Pdfa,
    Pdf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct EnqueuePayload {
    pub files: Vec<String>,
    pub output_dir: String,
    pub options: OcrOptions,
    pub processing: Option<ocr_engine::types::ProcessingConfigInput>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
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
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct QueueState {
    pub jobs: Vec<Job>,
    pub is_running: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
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

pub(crate) static START_TIME: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
pub(crate) static RUNTIME: std::sync::OnceLock<Arc<RuntimeResources>> = std::sync::OnceLock::new();

pub fn resolve_tessdata_path() -> Option<String> {
    use std::path::PathBuf;
    [
        std::env::current_dir().ok().map(|d| d.join("tessdata")),
        std::env::var("TESSDATA_PREFIX").ok().map(PathBuf::from),
        Some(PathBuf::from("/opt/homebrew/share/tessdata")),
        Some(PathBuf::from("/usr/local/share/tessdata/")),

    ]
    .into_iter()
    .flatten()
    .find(|p| p.exists())
    .map(|p| p.to_string_lossy().to_string())
}

pub fn resolve_pdfium_path(app: &tauri::AppHandle) -> Option<String> {
    use std::path::PathBuf;
    [
        std::env::var("PDFIUM_LIB_PATH").ok().map(PathBuf::from),
        app.path()
            .resource_dir()
            .ok()
            .map(|d| d.join("pdfium").join(pdfium_lib_name())),
        Some(PathBuf::from("/opt/homebrew/lib/libpdfium.dylib")),

    ]
    .into_iter()
    .flatten()
    .find(|p| p.exists())
    .map(|p| p.to_string_lossy().to_string())
}

fn pdfium_lib_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "libpdfium.dylib"
    } else {
        "pdfium.dll"
    }
}

#[cfg(feature = "ocr")]
fn seed_tess_pool(pool: ocr_engine::engine::SharedTessPool) {
    let tessdata_path = resolve_tessdata_path();
    let Some(path) = tessdata_path else {
        tracing::warn!(target: "knox::ocr", "tessdata path not found — pool will create TessApi lazily");
        return;
    };
    std::thread::spawn(
        move || match crate::ocr_engine::ocr::TessApi::new(&path, "eng") {
            Ok(tess) => {
                tracing::info!(target: "knox::ocr", "seeding TessApi pool with warm instance");
                let mut guard = pool.lock().expect("tess pool lock poisoned");
                *guard = Some(tess);
            }
            Err(e) => {
                tracing::warn!(target: "knox::ocr", error = %e, "TessApi warmup failed");
            }
        },
    );
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(if cfg!(debug_assertions) {
                    tracing_subscriber::filter::LevelFilter::DEBUG.into()
                } else {
                    tracing_subscriber::filter::LevelFilter::INFO.into()
                })
                .from_env_lossy(),
        )
        .with_writer(std::io::stderr)
        .init();
    START_TIME.get_or_init(Instant::now);
    tracing::info!("application started");
    tauri::Builder::default()
        .plugin(dialog_init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .manage(Arc::new(Mutex::new(queue::QueueStore::default())))
        .manage(Arc::new(Mutex::new(HistoryStore::default())))
        .setup(|app| {
            // Defer history load to background — it's disk I/O that doesn't
            // block the first paint.
            let history_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let history_state: tauri::State<SharedHistory> = history_handle.state();
                let loaded = history::load_history(&history_handle);
                if let Ok(mut history) = history_state.lock() {
                    *history = loaded;
                    tracing::info!(target: "knox::history", entries = history.entries.len(), "history loaded");
                }
            });

            #[cfg(feature = "ocr")]
            {
                let tess_pool: ocr_engine::engine::SharedTessPool =
                    Arc::new(Mutex::new(None));
                seed_tess_pool(tess_pool.clone());
                app.manage(tess_pool);
            }
            #[cfg(not(feature = "ocr"))]
            {
                let tess_pool: ocr_engine::engine::SharedTessPool =
                    Arc::new(Mutex::new(()));
                app.manage(tess_pool);
            }

            // Create PdfiumEngine with runtime dylib loading.
            // If the dylib is not found, it falls back to lopdf extraction silently.
            let pdfium_path = crate::resolve_pdfium_path(app.handle()).unwrap_or_default();
            let pdfium_engine = Arc::new(ocr_engine::render::PdfiumEngine::new(&pdfium_path));
            app.manage(pdfium_engine);

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
            commands::get_file_metadata,
            commands::log_window_shown,
            commands::ensure_language_packs,
            commands::check_file_encrypted,
            commands::restart_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
