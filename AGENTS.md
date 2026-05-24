# Knox — AI Agent Context

## Project
Offline desktop app for batch OCR, cleaning, and compression of PDFs using a Rust-native pipeline (Tesseract FFI).

## Tech Stack
- **Frontend**: React 19, TypeScript 5.8, Tailwind CSS 4, shadcn/ui, Vite 7
- **Desktop**: Tauri v2 (Rust backend)
- **Rust**: Edition 2024, tokio (async), rayon (CPU parallelism), tesseract-sys/leptonica-sys (OCR FFI), lopdf (PDF), image/imageproc (preprocessing)
- **Package**: pnpm 10+, Node 22+

## Architecture
```
UI (React) → Tauri invoke() → Rust Commands → Queue → OCR Engine (Tesseract FFI)
                                                                ↓
Events: pipeline-progress, jobProgress, jobFinished, queueState, historyUpdated
```

## Commands
```bash
pnpm install           # Install JS deps
pnpm tauri dev         # Dev server + Tauri window
pnpm build             # TypeScript check + Vite build
pnpm test              # Vitest frontend tests
cargo test             # Rust tests (in src-tauri/)
pnpm tauri build       # Production build
```

## Code Conventions

### TypeScript/React
- Import alias `@/*` maps to `./src/*`
- Components in `src/components/`, shadcn primitives in `src/components/ui/`
- Types in `src/types.ts`
- Event listeners use `listen<T>()` from `@tauri-apps/api/event`
- Tauri commands called via `invoke()` from `@tauri-apps/api/core`

### Rust
- Snake_case for functions/variables, PascalCase for types
- `#[serde(rename_all = "camelCase")]` on all frontend-facing structs
- `#[serde(rename_all = "lowercase")]` on enums (JobStatus, OutputType)
- `PipelineError` enum with thiserror for all OCR pipeline errors
- FFI calls wrapped in `catch_unwind` for panic isolation

## Key Files
| Path | Purpose |
|---|---|
| `src/App.tsx` | Main component: state, event wiring, Tauri API calls |
| `src/types.ts` | Shared TypeScript types |
| `src-tauri/src/lib.rs` | All Tauri commands (11 total) |
| `src-tauri/src/security.rs` | Path validation |
| `src-tauri/src/ocr_engine/` | Full OCR pipeline (11 modules) |
| `docs/spec.md` | Product spec |
| `docs/architecture.md` | Architecture overview |
| `docs/specs/` | Granular component/module specs |

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
| `set_runner_path` | `path: String` | `RunnerStatus` | Set sidecar path |
| `get_runner_status` | — | `RunnerStatus` | Sidecar config status |
| `write_log_file` | `path, content: String` | `()` | Write log to disk |
| `get_file_metadata` | `path: String` | `FileMetadata` | Get file size |

## Tauri Events
| Event | Payload | Fires |
|---|---|---|
| `pipeline-progress` | `PipelineProgress` | Per-page progress during OCR |
| `queueState` | `QueueState` | Queue start/stop/empty |
| `jobProgress` | `Job` | Job status change |
| `jobFinished` | `Job` | Job completed/failed/cancelled |
| `historyUpdated` | `Vec<HistoryEntry>` | History modified |

## Performance Targets
- **TTI (Time to Interactive)**: Must be under 3 seconds (measured by `knox::startup` log `elapsed_ms`)
- Event listeners lazily registered on first job start (not on mount)
- No blocking IPC calls during initial render (`get_status` deferred)
- Greeting is a compile-time constant (no timer/re-renders)

## Development Workflow

For every new feature or change, follow this order:

1. **Specs** — Define *what* needs to exist. Write/update a granular spec in `docs/specs/` with user journeys and acceptance criteria.
2. **Tests** — Define *correctness* before implementation. Write failing tests that validate the spec's acceptance criteria (Vitest for frontend, `#[cfg(test)]` for Rust).
3. **Code** — Implement *just enough* to make tests pass (Red-Green-Refactor). Stick to existing patterns — don't add new frameworks or dependencies.
4. **Docs** — Document *after* the implementation settles. Add `///` / `/** */` doc comments on all new public items. Update `AGENTS.md` if commands, events, or architecture changed.
5. **Verify** — Run all three gates: `pnpm test && cargo test && pnpm build`. All must pass. Fix warnings.

## OCR Pipeline Modules
| Module | Responsibility |
|---|---|
| `config.rs` | Semaphore capacity calculation |
| `runtime.rs` | Rayon thread pool + file semaphore |
| `ingest.rs` | Bounded channel file ingestion |
| `engine.rs` | Pipeline orchestrator (`process_files`) |
| `image.rs` | Preprocessing: denoise → binarize → deskew → bitonal |
| `ocr.rs` | Safe Tesseract FFI wrapper (with panic isolation) |
| `pdf.rs` | Load, extract, encode (CCITT G4/Flate), replace, save |
| `progress.rs` | Atomic progress tracker → Tauri events |
| `types.rs` | Shared schema (enums, config structs) |
| `error.rs` | `PipelineError` enum |
