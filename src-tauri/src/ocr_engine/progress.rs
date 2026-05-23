use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use crate::ocr_engine::types::{PipelineProgress, PipelineStatus};

#[derive(Clone)]
pub struct ProgressTracker {
    totals: Arc<Totals>,
}

struct Totals {
    total_files_processed: AtomicU32,
    total_files_in_queue: AtomicU32,
    total_pages_processed: AtomicU32,
    total_page_time_ms: AtomicU64,
}

impl ProgressTracker {
    pub fn new(total_files_in_queue: u32) -> Self {
        Self {
            totals: Arc::new(Totals {
                total_files_processed: AtomicU32::new(0),
                total_files_in_queue: AtomicU32::new(total_files_in_queue),
                total_pages_processed: AtomicU32::new(0),
                total_page_time_ms: AtomicU64::new(0),
            }),
        }
    }

    pub fn record_page_time(&self, ms: u64) {
        self.totals
            .total_pages_processed
            .fetch_add(1, Ordering::Relaxed);
        self.totals
            .total_page_time_ms
            .fetch_add(ms, Ordering::Relaxed);
    }

    pub fn record_file_done(&self) {
        self.totals
            .total_files_processed
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn emit(
        &self,
        app: &AppHandle,
        job_id: String,
        status: PipelineStatus,
        current_page: u32,
        total_pages: u32,
        error_message: Option<String>,
    ) {
        let pages = self.totals.total_pages_processed.load(Ordering::Relaxed);
        let total_time = self.totals.total_page_time_ms.load(Ordering::Relaxed);
        let avg = if pages == 0 {
            0
        } else {
            total_time / pages as u64
        };
        let payload = PipelineProgress {
            job_id,
            status,
            current_page,
            total_pages,
            total_files_processed: self.totals.total_files_processed.load(Ordering::Relaxed),
            total_files_in_queue: self.totals.total_files_in_queue.load(Ordering::Relaxed),
            average_ms_per_page: avg,
            error_message,
        };
        let _ = app.emit("pipeline-progress", payload);
    }
}
