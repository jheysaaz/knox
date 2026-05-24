use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Instant;

use tauri::{AppHandle, Emitter};

use crate::ocr_engine::types::{PipelineProgress, PipelineStatus};

const MIN_EMIT_INTERVAL_MS: u64 = 50;

/// Tracks per-file and aggregate pipeline progress and emits `pipeline-progress` events.
/// Events are rate-limited to one per `MIN_EMIT_INTERVAL_MS` to avoid flooding the frontend.
#[derive(Clone)]
pub struct ProgressTracker {
    totals: Arc<Totals>,
    last_emit: Arc<std::sync::Mutex<Option<Instant>>>,
}

struct Totals {
    total_files_processed: AtomicU32,
    total_files_in_queue: AtomicU32,
    total_pages_processed: AtomicU32,
    total_page_time_ms: AtomicU64,
}

impl ProgressTracker {
    /// Creates a new tracker initialized for `total_files_in_queue` files.
    pub fn new(total_files_in_queue: u32) -> Self {
        Self {
            totals: Arc::new(Totals {
                total_files_processed: AtomicU32::new(0),
                total_files_in_queue: AtomicU32::new(total_files_in_queue),
                total_pages_processed: AtomicU32::new(0),
                total_page_time_ms: AtomicU64::new(0),
            }),
            last_emit: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /// Records the time (ms) spent processing one page and increments the page counter.
    pub fn record_page_time(&self, ms: u64) {
        self.totals
            .total_pages_processed
            .fetch_add(1, Ordering::Relaxed);
        self.totals
            .total_page_time_ms
            .fetch_add(ms, Ordering::Relaxed);
    }

    /// Increments the completed-files counter.
    pub fn record_file_done(&self) {
        self.totals
            .total_files_processed
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Emits a `pipeline-progress` event to the Tauri frontend with current job stats.
    /// Rate-limited to one event per `MIN_EMIT_INTERVAL_MS` to avoid flooding the frontend.
    pub fn emit(
        &self,
        app: &AppHandle,
        job_id: String,
        status: PipelineStatus,
        current_page: u32,
        total_pages: u32,
        error_message: Option<String>,
    ) {
        {
            let mut last = self.last_emit.lock().unwrap();
            if let Some(t) = *last
                && t.elapsed().as_millis() < MIN_EMIT_INTERVAL_MS as u128
            {
                return;
            }
            *last = Some(Instant::now());
        }

        let pages = self.totals.total_pages_processed.load(Ordering::Relaxed);
        let total_time = self.totals.total_page_time_ms.load(Ordering::Relaxed);
        let avg = if pages == 0 {
            0
        } else {
            total_time / pages as u64
        };
        let payload = PipelineProgress {
            job_id: job_id.clone(),
            status,
            current_page,
            total_pages,
            total_files_processed: self.totals.total_files_processed.load(Ordering::Relaxed),
            total_files_in_queue: self.totals.total_files_in_queue.load(Ordering::Relaxed),
            average_ms_per_page: avg,
            error_message,
        };
        tracing::trace!(target: "knox::progress", ?status, current_page, total_pages, "progress emit");
        if let Err(e) = app.emit("pipeline-progress", payload) {
            tracing::warn!(target: "knox::progress", "emit failed: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tracker_starts_at_zero() {
        let t = ProgressTracker::new(5);
        assert_eq!(t.totals.total_files_in_queue.load(Ordering::Relaxed), 5);
        assert_eq!(t.totals.total_files_processed.load(Ordering::Relaxed), 0);
        assert_eq!(t.totals.total_pages_processed.load(Ordering::Relaxed), 0);
        assert_eq!(t.totals.total_page_time_ms.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn record_page_time_increments_counters() {
        let t = ProgressTracker::new(1);
        t.record_page_time(100);
        t.record_page_time(200);
        assert_eq!(t.totals.total_pages_processed.load(Ordering::Relaxed), 2);
        assert_eq!(t.totals.total_page_time_ms.load(Ordering::Relaxed), 300);
    }

    #[test]
    fn record_file_done_increments() {
        let t = ProgressTracker::new(3);
        t.record_file_done();
        assert_eq!(t.totals.total_files_processed.load(Ordering::Relaxed), 1);
        t.record_file_done();
        assert_eq!(t.totals.total_files_processed.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn average_is_zero_when_no_pages() {
        let t = ProgressTracker::new(1);
        // emit would compute avg = 0 / 0 = 0 (no panic)
        let pages = t.totals.total_pages_processed.load(Ordering::Relaxed);
        let time = t.totals.total_page_time_ms.load(Ordering::Relaxed);
        let avg = if pages == 0 { 0 } else { time / pages as u64 };
        assert_eq!(avg, 0);
    }
}
