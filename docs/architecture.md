# Architecture

## High-level Components
- UI (React + Tailwind + shadcn/ui)
- Core (Rust, Tauri commands)
- OCR sidecar bundle (Python + OCRmyPDF + native deps)

## Data Flow
1) User selects PDFs and output folder.
2) UI enqueues jobs via Tauri command.
3) Rust queue spawns sidecar per job.
4) Sidecar logs are parsed into progress events.
5) UI renders progress and completion states.
6) History is written locally after job completion.

## Tauri Command API
- `enqueue(files, outputDir, options)`
- `startQueue()`
- `pauseQueue()`
- `cancelJob(id)`
- `clearQueue()`
- `getStatus()`
- `getHistory()`
- `clearHistory()`

## Event Types
- `queueState`
- `jobProgress`
- `jobLog`
- `jobFinished`
- `jobFailed`
- `historyUpdated`

## Concurrency Strategy
- Default: `min(2, physical_cores / 2)` concurrent jobs.
- If safe mode enabled: 1 concurrent job.
- Per-job `--jobs` value:
  - If multiple concurrent jobs: 1-2
  - If single job: cores - 1

## Sidecar Bundling
- OS-specific bundles stored under `resources/`.
- Rust resolves the resource path using Tauri APIs.
