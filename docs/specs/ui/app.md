# App — Main Component Spec

## User Journey
1. User opens the app → sees greeting, file drop zone, output directory picker, profile selector
2. User drags/clicks to add PDFs → files appear in queue panel
3. User selects output directory
4. User clicks "Start OCR Processing" → files are enqueued, processing begins
5. Progress events update file statuses in real-time
6. On completion, files show "Complete" status; history is recorded
7. User can pause, remove, or reprocess files

## Props
None (top-level component)

## State
- `files: FileItem[]` — list of files in queue
- `outputDir: string` — selected output directory
- `logs: LogEntry[]` — activity log entries
- `showActivity: boolean` — toggle activity panel
- `greeting: string` — time-based greeting
- `settings: ProfileValues` — OCR settings profile

## Events Handled
- `pipeline-progress` → updates per-file progress %
- `queueState` → syncs file statuses with backend queue
- `jobFinished` → marks file complete/failed, adds log entry
- `jobProgress` → updates file status

## Commands Called
- `get_status` (on mount) — restore queue state
- `enqueue` — add files for processing
- `start_queue` — begin processing
- `pause_queue` — pause processing
- `remove_job` — remove single job
- `clear_queue` — clear all jobs

## Acceptance Criteria
- Renders all child components (Header, FileDropZone, OutputDirectory, AdvancedOptions, QueueView, LogPanel)
- Shows error toast when starting with empty queue
- Shows error toast when starting without output directory
- Disables "Add to Queue" state appropriately when running
- Updates file progress from Tauri events
- Loads existing queue state on mount
