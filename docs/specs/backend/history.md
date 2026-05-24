# History System Spec

## Data Structure
```rust
struct HistoryStore {
    entries: Vec<HistoryEntry>,
}

struct HistoryEntry {
    id: String,
    input_path: String,
    output_path: String,
    status: JobStatus,
    started_at: u64,
    finished_at: u64,
    duration_ms: u64,
    options: OcrOptions,
}
```

## Persistence
- File: `<app_data_dir>/history.json`
- Format: JSON array of HistoryEntry
- Retention: 100 entries max (newest first)
- Loaded on app startup, written on each modification

## Commands
| Command | Effect |
|---|---|
| `get_history` | Returns current entries |
| `clear_history` | Clears entries, writes empty file, emits `historyUpdated` |

## Events
- `historyUpdated` emitted after every push and after clear

## Acceptance Criteria
- History persists across app restarts
- Entries are prepended (newest first)
- History is truncated to 100 entries max
- Clear empties both memory and disk
- Invalid history file on disk defaults to empty
