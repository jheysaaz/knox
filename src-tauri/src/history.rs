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

pub fn save_history(app: &AppHandle, store: &HistoryStore) -> Result<(), String> {
    let path = history_path(app)?;
    let data = serde_json::to_string_pretty(store)
        .map_err(|e| format!("Unable to serialize history: {e}"))?;
    std::fs::write(&path, &data)
        .map_err(|e| format!("Unable to write history to {}: {e}", path.display()))?;
    tracing::debug!(target: "knox::history", path = %path.display(), entries = store.entries.len(), "history saved");
    Ok(())
}

pub fn push_history(app: &AppHandle, state: &Arc<Mutex<HistoryStore>>, entry: HistoryEntry) {
    let mut history = match state.lock() {
        Ok(lock) => lock,
        Err(e) => {
            tracing::error!(target: "knox::history", "lock poisoned: {e}");
            return;
        }
    };
    history.entries.insert(0, entry);
    if history.entries.len() > HISTORY_LIMIT {
        history.entries.truncate(HISTORY_LIMIT);
    }
    if let Err(e) = save_history(app, &history) {
        tracing::error!(target: "knox::history", "save failed: {e}");
    }
    if let Err(e) = app.emit("historyUpdated", history.entries.clone()) {
        tracing::error!(target: "knox::history", "emit failed: {e}");
    }
}
