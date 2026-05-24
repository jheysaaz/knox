# Queue System Spec

## Data Structure
```rust
struct QueueStore {
    jobs: Vec<Job>,
    queue: VecDeque<usize>,    // indices into jobs
    is_running: bool,
    in_flight: usize,
    cancelled: Arc<AtomicBool>,
}
```

## Job Lifecycle
```
Enqueue → Queued → Running → Completed
                          → Failed
                          → Cancelled
         → Remove (only from Queued or Cancelled)
```

## Concurrency
- Default: `min(2, physical_cores / 2)` concurrent jobs
- Safe mode: 1 concurrent job
- Controlled by `max_concurrency` in OcrOptions

## Commands
| Command | Effect |
|---|---|
| `enqueue` | Validates output dir, creates Job per file, appends to queue |
| `start_queue` | Spawns async loop that pops jobs and processes via OCR engine |
| `pause_queue` | Sets isRunning=false, cancels in-flight jobs |
| `remove_job` | Removes from jobs[] and queue[] by index (only queued/cancelled) |
| `clear_queue` | Clears all jobs and resets counters |
| `get_status` | Returns snapshot of all jobs + running state |

## Events
- `queueState` emitted on start, pause, and when queue drains (in_flight=0)
- `jobProgress` emitted on status transitions
- `jobFinished` emitted on terminal states

## Acceptance Criteria
- Enqueue with valid path adds job
- Enqueue with invalid output dir fails
- Remove removes job and adjusts indices
- Remove running job fails
- Clear empties queue
- Double start_queue is no-op
- Pause sets isRunning=false and cancels
- Queue drains correctly when all jobs complete
