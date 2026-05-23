use std::num::NonZeroUsize;

use crate::ocr_engine::types::ProcessingConfig;

pub fn effective_max_concurrent_files(config: &ProcessingConfig) -> usize {
    if let Some(value) = config.max_concurrent_files {
        return value.max(1);
    }

    let cores = std::thread::available_parallelism()
        .map(NonZeroUsize::get)
        .unwrap_or(1);
    (cores / 2).max(1)
}
