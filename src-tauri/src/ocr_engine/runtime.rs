use std::num::NonZeroUsize;

use rayon::ThreadPool;
use rayon::ThreadPoolBuilder;
use tokio::sync::Semaphore;

use crate::ocr_engine::config::effective_max_concurrent_files;
use crate::ocr_engine::types::ProcessingConfig;

/// Shared OCR runtime: a Rayon thread pool and a semaphore governing concurrent file processing.
#[derive(Clone)]
pub struct RuntimeResources {
    pub pool: std::sync::Arc<ThreadPool>,
    pub file_semaphore: std::sync::Arc<Semaphore>,
}

/// Builds a `RuntimeResources` from config: creates a Rayon thread pool sized to
/// `thread_pool_size` (or `cores - 2`, minimum 1) and a semaphore with permits
/// determined by `effective_max_concurrent_files`.
pub fn build_runtime(config: &ProcessingConfig) -> RuntimeResources {
    let threads = config.thread_pool_size.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(NonZeroUsize::get)
            .unwrap_or(1)
            .saturating_sub(2)
            .max(1)
    });

    let pool = ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("rayon pool creation failed");

    let permits = effective_max_concurrent_files(config);

    RuntimeResources {
        pool: std::sync::Arc::new(pool),
        file_semaphore: std::sync::Arc::new(Semaphore::new(permits)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(
        max_concurrent_files: Option<usize>,
        thread_pool_size: Option<usize>,
    ) -> ProcessingConfig {
        ProcessingConfig {
            max_concurrent_files,
            tessdata_path: "/tmp".to_string(),
            languages: "eng".to_string(),
            thread_pool_size,
        }
    }

    #[test]
    fn runtime_has_at_least_one_semaphore_permit() {
        let config = test_config(None, None);
        let rt = build_runtime(&config);
        assert!(rt.file_semaphore.available_permits() >= 1);
    }

    #[test]
    fn runtime_honors_semaphore_config() {
        let config = test_config(Some(3), None);
        let rt = build_runtime(&config);
        assert_eq!(rt.file_semaphore.available_permits(), 3);
    }

    #[test]
    fn runtime_pool_has_at_least_one_thread() {
        let config = test_config(None, Some(1));
        let rt = build_runtime(&config);
        // Rayon doesn't expose thread count directly, but ensure it runs
        let result = rt.pool.install(|| 42);
        assert_eq!(result, 42);
    }

    #[test]
    fn runtime_is_send_sync() {
        let config = test_config(None, None);
        let rt = build_runtime(&config);
        // Compile-time check: RuntimeResources must be Send + Sync (Clone uses Arc)
        fn assert_send<T: Send + Sync>() {}
        assert_send::<RuntimeResources>();
        let _ = rt; // suppress unused warning
    }
}
