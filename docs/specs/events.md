# Tauri Events Spec

## Event Catalog

### `pipeline-progress`
- **Payload**: `PipelineProgress`
- **When**: Per-page progress during OCR processing
- **Frequency**: Once per page per file
- **Fields**: jobId, status (processing|ocr|compressing|completed|failed), currentPage, totalPages, totalFilesProcessed, totalFilesInQueue, averageMsPerPage, errorMessage

### `queueState`
- **Payload**: `QueueState { jobs: Job[], isRunning: boolean }`
- **When**: Queue starts, stops, or becomes empty
- **Triggered by**: `start_queue`, `pause_queue`, queue draining

### `jobProgress`
- **Payload**: `Job`
- **When**: Job status transitions (queued → running → completed/failed/cancelled)
- **Fields**: id, inputPath, outputPath, status, percent, startedAt, finishedAt, errorMessage

### `jobFinished`
- **Payload**: `Job` (terminal state)
- **When**: Job completes, fails, or is cancelled
- **Always emitted after** `jobProgress` for the same transition

### `historyUpdated`
- **Payload**: `Vec<HistoryEntry>`
- **When**: History is modified (new entry added or cleared)
- **Fields**: id, inputPath, outputPath, status, startedAt, finishedAt, durationMs

## Listener Pattern
```typescript
const unlisten = await listen<PayloadType>("event-name", (event) => {
  // handle event.payload
});
// cleanup:
unlisten();
```
