# Runtime Module Spec

## Struct: `RuntimeResources`
Bundles Rayon thread pool + tokio Semaphore for resource-throttled processing.

```
RuntimeResources {
    pool: Arc<ThreadPool>,       // CPU-bound thread pool
    file_semaphore: Arc<Semaphore>,  // backpressure for concurrent files
}
```

## `build_runtime(config) -> RuntimeResources`
- Thread count: `config.thread_pool_size` or `max(1, available_parallelism - 2)`
- Semaphore permits: `config.max_concurrent_files` or `effective_max_concurrent_files(config)`

## Acceptance Criteria
- Pool always has at least 1 thread
- Semaphore always has at least 1 permit
- Config values are honored when provided
- Clone is cheap (Arc-based)
