# Architecture

## High-level Components
- **UI** (React 19 + TypeScript + Tailwind CSS 4 + shadcn/ui)
- **Core** (Rust, Tauri v2 commands)
- **OCR Pipeline** (Rust-native: tesseract-sys FFI, lopdf, image/imageproc, rayon)

## Data Flow
1. User selects PDFs and output folder in the UI.
2. UI enqueues jobs via `invoke("enqueue", ...)` Tauri command.
3. Rust backend spawns an async pipeline per file, bounded by a semaphore.
4. Per-page progress is emitted as `pipeline-progress` events.
5. UI renders progress and completion states.
6. History is written locally after job completion.

## Tauri Command API
| Command | Params | Returns | Description |
|---|---|---|---|
| `enqueue` | `payload: EnqueuePayload` | `QueueState` | Add files to processing queue |
| `start_queue` | — | `()` | Begin processing queued jobs |
| `pause_queue` | — | `()` | Pause all processing |
| `remove_job` | `job_id: String` | `QueueState` | Remove a queued job |
| `clear_queue` | — | `QueueState` | Clear all jobs |
| `get_status` | — | `QueueState` | Current queue state |
| `get_history` | — | `Vec<HistoryEntry>` | Job history |
| `clear_history` | — | `()` | Clear history |
| `write_log_file` | `path, content: String` | `()` | Write log to disk |
| `get_file_metadata` | `path: String` | `FileMetadata` | Get file size |
| `log_window_shown` | — | `()` | Log TTI measurement on first paint |
| `ensure_language_packs` | `languages: Vec<String>` | `LanguagePackResult` | Download missing Tesseract traineddata |

## Event Types
- `pipeline-progress` — Per-page progress during OCR
- `queueState` — Queue start/stop/empty
- `jobProgress` — Job status change
- `jobFinished` — Job completed/failed/cancelled
- `historyUpdated` — History modified

## Concurrency Strategy
- Rayon thread pool for CPU-bound image preprocessing (default: `cores - 2`, min 1).
- Async semaphore limits concurrent file processing (default: `max(1, cores / 2)`).
- Tokio runtime for async I/O and Tauri event emission.

## OCR Pipeline (Rust-native)
```
PDF load → read raw bytes
  → per page: try PdfiumEngine::render_page()
    → Ok(Some(img)): use pdfium raster (handles AcroForms, all filters)
    → Ok(None) / Err: fallback to lopdf XObject extraction
  → downsample → denoise →
binarize (Otsu/Bradley-Roth/Fixed) → morphology → deskew (Radon/Hough) →
Tesseract FFI OCR → compress (CCITT G4 / FlateDecode) →
replace image streams → save output
```

## Hybrid Rendering Architecture
| Module | Role |
|---|---|
| `render.rs` | PdfiumEngine wrapper with runtime dylib loading and fallback |
| `pdf.rs` | Lopdf extraction (fallback) + output encoding/saving |
| `engine.rs` | Hybrid loop: tries pdfium first, falls back per page |
