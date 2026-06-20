use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager};

use crate::HistoryEntry;
use crate::HistoryStore;

const HISTORY_LIMIT: usize = 100;

pub fn history_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "Unable to resolve app data directory".to_string())?;
    std::fs::create_dir_all(&dir).map_err(|_| "Unable to create app data directory".to_string())?;
    Ok(dir.join("history.json"))
}

pub fn load_history(app: &AppHandle) -> HistoryStore {
    let path = match history_path(app) {
        Ok(path) => path,
        Err(e) => {
            tracing::warn!(target: "knox::history", "cannot resolve history path: {e}");
            return HistoryStore::default();
        }
    };
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(_) => return HistoryStore::default(),
    };
    match serde_json::from_str(&contents) {
        Ok(store) => store,
        Err(e) => {
            tracing::error!(target: "knox::history", "corrupted history file, backing up: {e}");
            let backup = path.with_extension("json.corrupted");
            let _ = std::fs::rename(&path, &backup);
            HistoryStore::default()
        }
    }
}

pub(crate) fn save_history(app: &AppHandle, store: &HistoryStore) -> Result<(), String> {
    let path = history_path(app)?;
    atomic_save(&path, store)
}

fn atomic_save(path: &std::path::Path, store: &HistoryStore) -> Result<(), String> {
    let data = serde_json::to_string_pretty(store)
        .map_err(|e| format!("Unable to serialize history: {e}"))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &data)
        .map_err(|e| format!("Unable to write history to {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("Unable to rename history file: {e}"))?;
    tracing::debug!(target: "knox::history", path = %path.display(), "history saved atomically");
    Ok(())
}

pub fn push_history(app: &AppHandle, state: &Arc<Mutex<HistoryStore>>, entry: HistoryEntry) {
    push_history_in_memory(state, entry);
    let entries = match state.lock() {
        Ok(guard) => guard.entries.clone(),
        Err(e) => {
            tracing::error!(target: "knox::history", "lock poisoned: {e}");
            return;
        }
    };

    let app_handle = app.clone();
    let emit_entries = entries.clone();
    tauri::async_runtime::spawn(async move {
        let path = match history_path(&app_handle) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(target: "knox::history", "cannot resolve path: {e}");
                return;
            }
        };
        if let Err(e) =
            tokio::task::spawn_blocking(move || atomic_save(&path, &HistoryStore { entries }))
                .await
                .map_err(|e| format!("spawn_blocking failed: {e}"))
                .and_then(|r| r)
        {
            tracing::error!(target: "knox::history", "save failed: {e}");
        }

        if let Err(e) = app_handle.emit("historyUpdated", emit_entries) {
            tracing::error!(target: "knox::history", "emit failed: {e}");
        }
    });
}

/// Insert an entry into the in-memory store and enforce the limit (no I/O).
pub(crate) fn push_history_in_memory(state: &Arc<Mutex<HistoryStore>>, entry: HistoryEntry) {
    if let Ok(mut history) = state.lock() {
        history.entries.insert(0, entry);
        if history.entries.len() > HISTORY_LIMIT {
            history.entries.truncate(HISTORY_LIMIT);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HistoryEntry;

    fn fake_entry(id: &str) -> HistoryEntry {
        HistoryEntry {
            id: id.to_string(),
            input_path: format!("/input/{}.pdf", id),
            output_path: format!("/output/{}.pdf", id),
            status: crate::JobStatus::Completed,
            started_at: 1000,
            finished_at: 2000,
            duration_ms: 1000,
            options: crate::OcrOptions {
                output_type: crate::OutputType::Pdfa,
                safe_mode: false,
                max_concurrency: None,
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
                continue_on_error: false,
                password: None,
            },
        }
    }

    #[test]
    fn push_history_inserts_at_front() {
        let store = Arc::new(Mutex::new(HistoryStore::default()));
        push_history_in_memory(&store, fake_entry("a"));
        push_history_in_memory(&store, fake_entry("b"));

        let guard = store.lock().unwrap();
        assert_eq!(guard.entries.len(), 2);
        assert_eq!(guard.entries[0].id, "b");
        assert_eq!(guard.entries[1].id, "a");
    }

    #[test]
    fn push_history_truncates_at_limit() {
        let store = Arc::new(Mutex::new(HistoryStore::default()));
        for i in 0..HISTORY_LIMIT {
            push_history_in_memory(&store, fake_entry(&format!("e{i}")));
        }
        push_history_in_memory(&store, fake_entry("overflow"));

        let guard = store.lock().unwrap();
        assert_eq!(guard.entries.len(), HISTORY_LIMIT);
        assert_eq!(guard.entries[0].id, "overflow");
    }
}
