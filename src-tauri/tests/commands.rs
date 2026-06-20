#![cfg(all(feature = "integration", feature = "ocr"))]

use std::path::Path;
use std::sync::{Arc, Mutex};

use knox_lib::queue::{QueueStore, SharedQueue};
use knox_lib::{
    commands, EnqueuePayload, HistoryStore, JobStatus, OcrOptions, OutputType, SharedHistory,
};
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::Manager;

/// Creates a minimal Tauri app with managed state so `tauri::State` can be
/// extracted in tests.
fn build_test_app() -> tauri::App<tauri::test::MockRuntime> {
    mock_builder()
        .manage::<SharedQueue>(Arc::new(Mutex::new(QueueStore::default())))
        .manage::<SharedHistory>(Arc::new(Mutex::new(HistoryStore::default())))
        .build(mock_context(noop_assets()))
        .expect("failed to build test app")
}

/// Resolves an absolute path to a sample file.
fn sample_abs(name: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR has no parent");
    root.join("samples")
        .join(name)
        .to_string_lossy()
        .to_string()
}

/// Default options for test enqueues.
fn default_options() -> OcrOptions {
    OcrOptions {
        output_type: OutputType::Pdf,
        safe_mode: false,
        max_concurrency: None,
        binarization: knox_lib::ocr_engine::types::BinarizationMode::Otsu,
        fixed_threshold: 128,
        deskew_mode: knox_lib::ocr_engine::types::DeskewMode::Radon,
        denoise_level: 2,
        existing_text: knox_lib::ocr_engine::types::ExistingTextMode::Skip,
        psm: knox_lib::ocr_engine::types::PageSegMode::Auto,
        compression: knox_lib::ocr_engine::types::CompressionMode::Ccitt,
        resolution_dpi: 300,
        archive_enforcement: false,
        languages: Some("eng".into()),
        memory_pages: None,
        continue_on_error: false,
        password: None,
    }
}

fn default_enqueue(files: Vec<String>) -> EnqueuePayload {
    let tmp = std::env::temp_dir().join("knox-cmd-test-output");
    let _ = std::fs::create_dir_all(&tmp);
    EnqueuePayload {
        files,
        output_dir: tmp.to_string_lossy().to_string(),
        options: default_options(),
        processing: None,
    }
}

// ── Tests ──

#[test]
fn enqueue_single_file_returns_queued_state() {
    let app = build_test_app();
    let state = app.state::<SharedQueue>();

    let result = commands::enqueue(state, default_enqueue(vec![sample_abs("poster.pdf")]))
        .expect("enqueue failed");

    assert_eq!(result.jobs.len(), 1);
    assert!(matches!(result.jobs[0].status, JobStatus::Queued));
    assert!(!result.jobs[0].id.is_empty());
    assert!(result.jobs[0].output_path.contains("poster"));
    assert!(!result.is_running);
}

#[test]
fn enqueue_both_samples_returns_two_jobs() {
    let app = build_test_app();
    let state = app.state::<SharedQueue>();

    let result = commands::enqueue(
        state,
        default_enqueue(vec![sample_abs("poster.pdf"), sample_abs("skew.pdf")]),
    )
    .expect("enqueue failed");

    assert_eq!(result.jobs.len(), 2);
    assert!(matches!(result.jobs[0].status, JobStatus::Queued));
    assert!(matches!(result.jobs[1].status, JobStatus::Queued));
    assert_ne!(result.jobs[0].input_path, result.jobs[1].input_path);
    assert_ne!(result.jobs[0].output_path, result.jobs[1].output_path);
}

#[test]
fn enqueue_nonexistent_file_returns_error() {
    let app = build_test_app();
    let state = app.state::<SharedQueue>();

    let result = commands::enqueue(
        state,
        default_enqueue(vec!["/nonexistent/test.pdf".into()]),
    );

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind, "validation");
}

#[test]
fn get_status_after_enqueue_returns_jobs() {
    let app = build_test_app();
    let state = app.state::<SharedQueue>();

    commands::enqueue(state.clone(), default_enqueue(vec![sample_abs("poster.pdf")]))
        .expect("enqueue failed");

    let queue_state = commands::get_status(state).expect("get_status failed");
    assert_eq!(queue_state.jobs.len(), 1);
    assert!(matches!(queue_state.jobs[0].status, JobStatus::Queued));
}

#[test]
fn get_status_empty_initially() {
    let app = build_test_app();
    let state = app.state::<SharedQueue>();

    let queue_state = commands::get_status(state).expect("get_status failed");
    assert!(queue_state.jobs.is_empty());
    assert!(!queue_state.is_running);
}

#[test]
fn get_history_empty_initially() {
    let app = build_test_app();
    let history = app.state::<SharedHistory>();

    let entries = commands::get_history(history).expect("get_history failed");
    assert!(entries.is_empty());
}

#[test]
fn clear_queue_removes_all_jobs() {
    let app = build_test_app();
    let state = app.state::<SharedQueue>();

    commands::enqueue(state.clone(), default_enqueue(vec![sample_abs("poster.pdf")]))
        .expect("enqueue failed");

    let cleared = commands::clear_queue(state.clone()).expect("clear_queue failed");
    assert!(cleared.jobs.is_empty());

    let after = commands::get_status(state).expect("get_status failed");
    assert!(after.jobs.is_empty());
}

#[test]
fn check_file_encrypted_poster_returns_not_encrypted() {
    let result = commands::check_file_encrypted(sample_abs("poster.pdf"))
        .expect("check_file_encrypted failed");

    assert!(!result.encrypted);
    assert!(!result.file_id.is_empty());
}

#[test]
fn check_file_encrypted_nonexistent_returns_error() {
    let result = commands::check_file_encrypted("/nonexistent/file.pdf".into());
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind, "validation");
}

#[test]
fn remove_job_removes_queued_job() {
    let app = build_test_app();
    let state = app.state::<SharedQueue>();

    let enqueued = commands::enqueue(
        state.clone(),
        default_enqueue(vec![sample_abs("poster.pdf")]),
    )
    .expect("enqueue failed");

    let job_id = enqueued.jobs[0].id.clone();

    let after_remove = commands::remove_job(state.clone(), job_id).expect("remove_job failed");
    assert!(after_remove.jobs.is_empty());

    let after = commands::get_status(state).expect("get_status failed");
    assert!(after.jobs.is_empty());
}
