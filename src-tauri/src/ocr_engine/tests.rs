use super::runtime::build_runtime;
use super::types::ProcessingConfig;

#[test]
fn runtime_default_limits_nonzero() {
    let config = ProcessingConfig {
        max_concurrent_files: None,
        tessdata_path: "/tmp".to_string(),
        languages: "eng".to_string(),
        thread_pool_size: None,
    };
    let runtime = build_runtime(&config);
    assert!(runtime.file_semaphore.available_permits() >= 1);
}
