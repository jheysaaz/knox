# Progress Module Spec

## Struct: `ProgressTracker`
Atomic progress tracker that emits Tauri events.

```
ProgressTracker {
    total_files_processed: AtomicU32,
    total_files_in_queue: AtomicU32,
    total_pages_processed: AtomicU32,
    total_page_time_ms: AtomicU64,
}
```

## Methods
- `new(total_files_in_queue)` — initializes with queue size
- `record_page_time(ms)` — increments pages counter, adds time
- `record_file_done()` — increments files counter
- `emit(app, job_id, status, current_page, total_pages, error)` — emits `pipeline-progress` event
  - Computes average_ms_per_page = total_time / pages (0 if no pages)
  - Includes all counter values in payload

## Acceptance Criteria
- New tracker starts with zero counters except total_files_in_queue
- record_page_time increments pages and accumulates time
- record_file_done increments files processed
- emit computes correct average (including division by zero safe)
- emit sends event with all expected fields
