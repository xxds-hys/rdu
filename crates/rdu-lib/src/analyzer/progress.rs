//! Progress reporting for disk analysis.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Progress information for an ongoing scan.
#[derive(Debug, Clone)]
pub struct Progress {
    /// Number of items scanned
    pub item_count: u64,
    /// Total size scanned
    pub total_size: u64,
    /// Current path being scanned
    pub current_path: String,
    /// Whether the scan is complete
    pub done: bool,
}

impl Default for Progress {
    fn default() -> Self {
        Self::new()
    }
}

impl Progress {
    pub fn new() -> Self {
        Self {
            item_count: 0,
            total_size: 0,
            current_path: String::new(),
            done: false,
        }
    }
}

/// Thread-safe progress reporter.
#[derive(Debug)]
pub struct ProgressReporter {
    item_count: AtomicU64,
    total_size: AtomicU64,
    current_path: crossbeam::queue::ArrayQueue<String>,
    done: AtomicBool,
    start_time: Instant,
}

impl ProgressReporter {
    /// Create a new progress reporter.
    pub fn new() -> Self {
        Self {
            item_count: AtomicU64::new(0),
            total_size: AtomicU64::new(0),
            current_path: crossbeam::queue::ArrayQueue::new(16),
            done: AtomicBool::new(false),
            start_time: Instant::now(),
        }
    }

    /// Record a scanned item.
    pub fn record_item(&self, path: &str, size: u64) {
        self.item_count.fetch_add(1, Ordering::Relaxed);
        self.total_size.fetch_add(size, Ordering::Relaxed);

        // Try to update current path (non-blocking)
        let _ = self.current_path.push(path.to_string());
        // If queue is full, pop one and push
        if self.current_path.push(path.to_string()).is_err() {
            let _ = self.current_path.pop();
            let _ = self.current_path.push(path.to_string());
        }
    }

    /// Mark the scan as complete.
    pub fn finish(&self) {
        self.done.store(true, Ordering::Release);
    }

    /// Get the current progress.
    pub fn get_progress(&self) -> Progress {
        Progress {
            item_count: self.item_count.load(Ordering::Relaxed),
            total_size: self.total_size.load(Ordering::Relaxed),
            current_path: self.current_path.pop().unwrap_or_else(|| String::new()),
            done: self.done.load(Ordering::Acquire),
        }
    }

    /// Get the elapsed time since the scan started.
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Check if the scan is done.
    pub fn is_done(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }
}

impl Default for ProgressReporter {
    fn default() -> Self {
        Self::new()
    }
}
