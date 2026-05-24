use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::Job;

pub struct QueueStore {
    pub jobs: Vec<Job>,
    pub queue: VecDeque<usize>,
    pub is_running: bool,
    pub in_flight: usize,
    pub cancelled: Arc<AtomicBool>,
}

impl Default for QueueStore {
    fn default() -> Self {
        Self {
            jobs: Vec::new(),
            queue: VecDeque::new(),
            is_running: false,
            in_flight: 0,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
}

pub type SharedQueue = Arc<Mutex<QueueStore>>;

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_millis() as u64
}

pub fn default_concurrency() -> usize {
    let cores = num_cpus::get_physical().max(1);
    let half = (cores / 2).max(1);
    std::cmp::min(2, half)
}
