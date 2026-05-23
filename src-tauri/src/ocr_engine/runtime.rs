use std::num::NonZeroUsize;

use rayon::ThreadPool;
use rayon::ThreadPoolBuilder;
use tokio::sync::Semaphore;

use crate::ocr_engine::config::effective_max_concurrent_files;
use crate::ocr_engine::types::ProcessingConfig;

#[derive(Clone)]
pub struct RuntimeResources {
    pub pool: std::sync::Arc<ThreadPool>,
    pub file_semaphore: std::sync::Arc<Semaphore>,
}

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
