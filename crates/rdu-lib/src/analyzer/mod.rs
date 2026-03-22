//! Disk usage analyzer module.

mod parallel;
mod progress;
mod sequential;

pub use parallel::ParallelAnalyzer;
pub use progress::{Progress, ProgressReporter};
pub use sequential::SequentialAnalyzer;

use crate::fs::{Dir, DirRef, File, Item, ItemFlag, ItemRef};
use crate::ignore::IgnoreMatcher;
use crate::platform::FileMetadata;
use crate::timefilter::TimeFilter;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

/// Result of analyzing a directory.
pub type AnalysisResult = Result<DirRef, AnalysisError>;

/// Errors that can occur during analysis.
#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path does not exist: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Not a directory: {0}")]
    NotADirectory(String),
}

/// Configuration for the analyzer.
#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    /// Follow symbolic links for files
    pub follow_symlinks: bool,
    /// Don't cross filesystem boundaries
    pub no_cross: bool,
    /// Ignore matcher
    pub ignore: IgnoreMatcher,
    /// Maximum number of threads (0 = auto)
    pub max_threads: usize,
    /// Time filter for file modification times
    pub time_filter: TimeFilter,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            follow_symlinks: false,
            no_cross: false,
            ignore: IgnoreMatcher::new(),
            max_threads: 0,
            time_filter: TimeFilter::default(),
        }
    }
}

/// Create a file item from metadata.
pub fn create_file(name: String, meta: &FileMetadata) -> ItemRef {
    let flag = if meta.is_symlink {
        ItemFlag::Symlink
    } else {
        ItemFlag::Normal
    };

    Arc::new(
        File::new(name)
            .with_size(meta.size)
            .with_usage(meta.usage)
            .with_mtime(meta.mtime)
            .with_flag(flag)
            .with_inode(if meta.nlink > 1 { meta.inode } else { 0 }),
    )
}

/// Create a directory item from metadata.
pub fn create_dir(name: String, meta: &FileMetadata) -> DirRef {
    let flag = if meta.is_symlink {
        ItemFlag::Symlink
    } else {
        ItemFlag::Normal
    };

    let dir = Arc::new(Dir::new(name));
    dir.set_size(meta.size);
    dir.set_usage(meta.usage);
    dir.set_mtime(meta.mtime);
    dir.set_flag(flag);
    dir.set_inode(if meta.nlink > 1 { meta.inode } else { 0 });
    dir
}

/// Track hard links to avoid counting them multiple times.
#[derive(Debug, Default)]
pub struct HardLinkTracker {
    seen_inodes: HashSet<u64>,
}

impl HardLinkTracker {
    pub fn new() -> Self {
        Self {
            seen_inodes: HashSet::new(),
        }
    }

    /// Check if an inode has been seen, and mark it as seen.
    /// Returns true if this is a new hard link.
    pub fn check_and_mark(&mut self, inode: u64) -> bool {
        if inode == 0 {
            return false;
        }
        self.seen_inodes.insert(inode)
    }

    /// Check if an inode has been seen.
    pub fn has_seen(&self, inode: u64) -> bool {
        inode != 0 && self.seen_inodes.contains(&inode)
    }

    /// Get all seen inodes.
    pub fn seen_inodes(&self) -> &HashSet<u64> {
        &self.seen_inodes
    }

    /// Clear all seen inodes.
    pub fn clear(&mut self) {
        self.seen_inodes.clear();
    }

    /// Get the count of seen inodes.
    pub fn len(&self) -> usize {
        self.seen_inodes.len()
    }

    /// Check if no inodes have been seen.
    pub fn is_empty(&self) -> bool {
        self.seen_inodes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_config_default() {
        let config = AnalyzerConfig::default();
        assert!(!config.follow_symlinks);
        assert!(!config.no_cross);
        assert_eq!(config.max_threads, 0);
        assert!(!config.time_filter.is_active());
    }

    #[test]
    fn test_analyzer_config_with_time_filter() {
        use std::time::Duration;

        let config = AnalyzerConfig {
            time_filter: TimeFilter::new().with_max_age(Duration::from_secs(3600)),
            ..Default::default()
        };
        assert!(config.time_filter.is_active());
    }

    #[test]
    fn test_hard_link_tracker_new() {
        let tracker = HardLinkTracker::new();
        assert!(tracker.is_empty());
        assert_eq!(tracker.len(), 0);
    }

    #[test]
    fn test_hard_link_tracker_check_and_mark() {
        let mut tracker = HardLinkTracker::new();

        // First time seeing inode 123
        assert!(tracker.check_and_mark(123));
        assert!(!tracker.is_empty());
        assert_eq!(tracker.len(), 1);

        // Second time seeing same inode
        assert!(!tracker.check_and_mark(123));
        assert_eq!(tracker.len(), 1);

        // New inode
        assert!(tracker.check_and_mark(456));
        assert_eq!(tracker.len(), 2);
    }

    #[test]
    fn test_hard_link_tracker_has_seen() {
        let mut tracker = HardLinkTracker::new();
        tracker.check_and_mark(100);
        tracker.check_and_mark(200);

        assert!(tracker.has_seen(100));
        assert!(tracker.has_seen(200));
        assert!(!tracker.has_seen(300));
        assert!(!tracker.has_seen(0)); // inode 0 is never "seen"
    }

    #[test]
    fn test_hard_link_tracker_clear() {
        let mut tracker = HardLinkTracker::new();
        tracker.check_and_mark(100);
        tracker.check_and_mark(200);
        assert_eq!(tracker.len(), 2);

        tracker.clear();
        assert!(tracker.is_empty());
    }

    #[test]
    fn test_hard_link_tracker_zero_inode() {
        let mut tracker = HardLinkTracker::new();

        // inode 0 should always return false
        assert!(!tracker.check_and_mark(0));
        assert!(!tracker.has_seen(0));
        assert!(tracker.is_empty());
    }

    #[test]
    fn test_create_file() {
        let meta = FileMetadata {
            size: 1024,
            usage: 4096,
            mtime: std::time::SystemTime::UNIX_EPOCH,
            inode: 12345,
            nlink: 2,
            is_dir: false,
            is_symlink: false,
            is_special: false,
        };

        let file = create_file("test.txt".to_string(), &meta);
        assert_eq!(file.name(), "test.txt");
        assert!(!file.is_dir());
        assert_eq!(file.size(), 1024);
        assert_eq!(file.usage(), 4096);
        assert_eq!(file.multi_link_inode(), 12345);
    }

    #[test]
    fn test_create_file_symlink() {
        let meta = FileMetadata {
            size: 100,
            usage: 100,
            mtime: std::time::SystemTime::UNIX_EPOCH,
            inode: 0,
            nlink: 1,
            is_dir: false,
            is_symlink: true,
            is_special: false,
        };

        let file = create_file("link".to_string(), &meta);
        assert_eq!(file.flag(), ItemFlag::Symlink);
    }

    #[test]
    fn test_create_dir() {
        let meta = FileMetadata {
            size: 0,
            usage: 4096,
            mtime: std::time::SystemTime::UNIX_EPOCH,
            inode: 0,
            nlink: 1,
            is_dir: true,
            is_symlink: false,
            is_special: false,
        };

        let dir = create_dir("mydir".to_string(), &meta);
        assert_eq!(dir.name(), "mydir");
        assert!(dir.is_dir());
        assert_eq!(dir.usage(), 4096);
    }

    #[test]
    fn test_analysis_error_display() {
        let err = AnalysisError::NotFound("/path/to/file".to_string());
        assert!(err.to_string().contains("/path/to/file"));

        let err = AnalysisError::PermissionDenied("/root".to_string());
        assert!(err.to_string().contains("Permission denied"));

        let err = AnalysisError::NotADirectory("/tmp/file.txt".to_string());
        assert!(err.to_string().contains("Not a directory"));
    }
}
