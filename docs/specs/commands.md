# Tauri Commands Spec

## Command Catalog

### `enqueue`
- **Params**: `payload: EnqueuePayload { files: string[], outputDir: string, options: OcrOptions, processing?: ProcessingConfigInput }`
- **Returns**: `QueueState`
- **Errors**: Invalid output directory (path validation)
- **Side effects**: Adds jobs to queue with Queued status

### `start_queue`
- **Params**: none
- **Returns**: `()`
- **Errors**: None
- **Side effects**: Spawns async queue processing loop; emits `queueState`, `jobProgress`, `jobFinished`, `pipeline-progress`

### `pause_queue`
- **Params**: none
- **Returns**: `()`
- **Errors**: None
- **Side effects**: Sets `isRunning=false`, sets cancellation flag; emits `queueState`

### `remove_job`
- **Params**: `jobId: String`
- **Returns**: `QueueState`
- **Errors**: Job not found, job is running
- **Side effects**: Removes job from queue

### `clear_queue`
- **Params**: none
- **Returns**: `QueueState`
- **Errors**: None
- **Side effects**: Clears all jobs and queue indices

### `get_status`
- **Params**: none
- **Returns**: `QueueState`
- **Errors**: None

### `get_history`
- **Params**: none
- **Returns**: `Vec<HistoryEntry>`
- **Errors**: None

### `clear_history`
- **Params**: none
- **Returns**: `()`
- **Errors**: Disk write failure
- **Side effects**: Clears in-memory history + writes to disk; emits `historyUpdated`

### `set_runner_path`
- **Params**: `path: String`
- **Returns**: `RunnerStatus`
- **Errors**: Path does not exist
- **Side effects**: Updates runner executable path

### `get_runner_status`
- **Params**: none
- **Returns**: `RunnerStatus`
- **Errors**: None

### `write_log_file`
- **Params**: `path: String`, `content: String`
- **Returns**: `()`
- **Errors**: Disk write failure

### `get_file_metadata`
- **Params**: `path: String`
- **Returns**: `FileMetadata { size: u64 }`
- **Errors**: File not found
