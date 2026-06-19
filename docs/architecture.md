# Architecture

## High-level Components
- **UI** (React 19 + TypeScript + Tailwind CSS 4 + shadcn/ui)
- **Core** (Rust, Tauri v2 commands)
- **OCR Pipeline** (Rust-native: tesseract-sys FFI, lopdf, image/imageproc, rayon)

## End-to-End Data Flow

```
User drops PDFs
  → FileDropZone filters .pdf files
  → Fetches metadata (get_file_metadata)
  → Adds to files[] state

User selects output dir
  → OutputDirectory.onChange
  → App updates outputDir state

User clicks "Start OCR Processing"
  → App validates: files.length > 0, outputDir !== ""
  → Maps settings to OcrOptions + ProcessingConfigInput
  → Invoke enqueue(payload)
  → Invoke start_queue()
  → Rust spawns async processing loop

Processing loop:
  → Pops job from queue
  → Emits jobProgress (status=Running)
  → Creates OCR Engine
  → Process each page:
      Emit pipeline-progress (status=Processing → Ocr → Compressing)
  → On success: emit jobFinished (status=Completed)
     Push to history, emit historyUpdated
  → On failure: emit jobFinished (status=Failed)
     Push error to history
  → On cancel: emit jobFinished (status=Cancelled)
     No history entry

UI handles events:
  → pipeline-progress: update file progress %
  → jobFinished: mark file complete/error/paused, add log
  → queueState: sync file statuses
  → historyUpdated: refresh history view
```

### Error Paths
1. **No files**: Toast "No files in queue" — no command called
2. **No output dir**: Toast "No output directory selected" — no command called
3. **Enqueue fails**: Toast error message, add error log
4. **Processing fails per file**: That file marked "Error", others continue
5. **Cancel during processing**: Current file stops, remaining stay queued

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
| `check_file_encrypted` | `path: String` | `FileEncryptionInfo` | Check if PDF is password-protected |
| `restart_app` | — | `()` | Restart the app (after update install) |

## Events

| Event | Payload | When |
|---|---|---|
| `pipeline-progress` | `PipelineProgress` | Per-page progress during OCR |
| `queueState` | `QueueState` | Queue starts, stops, or becomes empty |
| `jobProgress` | `Job` | Job status transitions |
| `jobFinished` | `Job` | Job completes, fails, or is cancelled |
| `historyUpdated` | `Vec<HistoryEntry>` | History is modified |

### Updater (frontend-only, no events)

| Call | Source | Description |
|---|---|---|
| `check()` | `@tauri-apps/plugin-updater` | Fetch latest.json, compare versions, return `Update \| null` |
| `update.downloadAndInstall(cb)` | `@tauri-apps/plugin-updater` | Download artifact with progress callback, then install |
| `getVersion()` | `@tauri-apps/api/app` | Current app version string |
| `invoke('restart_app')` | `@tauri-apps/api/core` | Restart app after install |

Listener pattern:
```typescript
const unlisten = await listen<PayloadType>("event-name", (event) => {
  // handle event.payload
});
```

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
|---|---|---|
| `render.rs` | PdfiumEngine wrapper with runtime dylib loading and fallback |
| `pdf.rs` | Lopdf extraction (fallback) + output encoding/saving |
| `engine.rs` | Hybrid loop: tries pdfium first, falls back per page |

## Updater

### How It Works
1. On app startup (and dev "Check Update" button), `checkForUpdates()` in `src/App.tsx` fires.
2. Calls `getVersion()` + `check()` (from `@tauri-apps/plugin-updater`) in parallel.
3. If update version is same "channel" (stable→stable, beta→beta, etc.) and newer, a sonner toast with "Download" action appears.
4. On download click, `downloadAndInstall(onEvent)` streams the artifact with progress events.
5. Progress updates the toast with a `<Progress>` bar.
6. On completion, toast switches to "Restart now" which invokes the `restart_app` command.

### Version Channel Filtering
Logic in `src/App.tsx:checkForUpdates`:
- Extract the pre-release tag from both versions (`1.0.0-beta.1` → `beta`)
- Stable (no tag) → only stable updates
- Pre-release → only matching channel (beta→beta, rc→rc, etc.)

### Local Testing

```bash
# Terminal 1 — start update server
./scripts/test-update-server.sh

# Terminal 2 — run app
pnpm tauri dev
```

The script:
1. Backs up `tauri.conf.json`, patches the updater endpoint to `http://localhost:9876/latest.json`
2. Creates a test `latest.json` with an auto-bumped version on the same channel
3. Generates a 10MB dummy DMG for download progress
4. Starts Python HTTP server on port 9876
5. On `Ctrl+C`, restores the original `tauri.conf.json`

The download will show progress bars but fail on signature mismatch (expected for dummy artifacts).

### Config (`tauri.conf.json`)
- `bundle.createUpdaterArtifacts: true` — generates `.sig` + update bundles during `pnpm tauri build`
- `plugins.updater.pubkey` — public key for signature verification
- `plugins.updater.endpoints` — URL to `latest.json` (GitHub Releases or local for dev)

### CI (`release.yml`)
- Sets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` from secrets
- `tauri-apps/tauri-action@v0` generates signed update artifacts automatically
