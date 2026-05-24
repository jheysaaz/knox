use std::num::NonZeroUsize;

use crate::ocr_engine::types::ProcessingConfig;

/// Returns the maximum number of files that can be processed concurrently.
///
/// Uses the explicit `max_concurrent_files` value if set, otherwise defaults to
/// half the available CPU cores (minimum 1).
#[allow(dead_code)]
pub fn effective_max_concurrent_files(config: &ProcessingConfig) -> usize {
    if let Some(value) = config.max_concurrent_files {
        return value.max(1);
    }

    let cores = std::thread::available_parallelism()
        .map(NonZeroUsize::get)
        .unwrap_or(1);
    (cores / 2).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_explicit_value() {
        let config = ProcessingConfig {
            max_concurrent_files: Some(5),
            tessdata_path: "/tmp".to_string(),
            languages: "eng".to_string(),
            thread_pool_size: None,
        };
        assert_eq!(effective_max_concurrent_files(&config), 5);
    }

    #[test]
    fn clamps_explicit_value_to_min_one() {
        let config = ProcessingConfig {
            max_concurrent_files: Some(0),
            tessdata_path: "/tmp".to_string(),
            languages: "eng".to_string(),
            thread_pool_size: None,
        };
        assert_eq!(effective_max_concurrent_files(&config), 1);
    }

    #[test]
    fn defaults_to_half_cores() {
        let config = ProcessingConfig {
            max_concurrent_files: None,
            tessdata_path: "/tmp".to_string(),
            languages: "eng".to_string(),
            thread_pool_size: None,
        };
        let result = effective_max_concurrent_files(&config);
        assert!(result >= 1, "should be at least 1");
        let cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        assert_eq!(result, (cores / 2).max(1));
    }
}
