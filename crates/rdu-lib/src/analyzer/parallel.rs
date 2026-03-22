//! Parallel disk usage analyzer.
//!
//! Optimized for maximum performance with:
//! - Batch directory reading (avoid redundant syscalls)
//! - True parallel directory traversal
//! - Minimal memory allocations
//! - Lock-free progress tracking

use super::{
    create_dir, create_file, AnalysisError, AnalysisResult, AnalyzerConfig, HardLinkTracker,
    ProgressReporter,
};
use crate::fs::{Dir, DirRef, ItemRef};
use crate::platform::{get_device_id, get_metadata, is_different_filesystem, metadata_to_file_metadata, FileMetadata};
use parking_lot::Mutex;
use rayon::prelude::*;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Parallel disk usage analyzer.
pub struct ParallelAnalyzer {
    config: AnalyzerConfig,
    progress: Arc<ProgressReporter>,
    hardlinks: Arc<Mutex<HardLinkTracker>>,
    root_dev: Option<u64>,
    items_scanned: AtomicU64,
    enable_progress: bool,
}

impl ParallelAnalyzer {
    /// Create a new parallel analyzer.
    pub fn new(config: AnalyzerConfig) -> Self {
        Self {
            config,
            progress: Arc::new(ProgressReporter::new()),
            hardlinks: Arc::new(Mutex::new(HardLinkTracker::new())),
            root_dev: None,
            items_scanned: AtomicU64::new(0),
            enable_progress: false,
        }
    }

    /// Enable progress reporting.
    pub fn with_progress(mut self, enable: bool) -> Self {
        self.enable_progress = enable;
        self
    }

    /// Analyze a directory.
    pub fn analyze(&mut self, path: &Path) -> AnalysisResult {
        if !path.exists() {
            return Err(AnalysisError::NotFound(path.display().to_string()));
        }

        let meta = get_metadata(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                AnalysisError::PermissionDenied(path.display().to_string())
            } else {
                AnalysisError::Io(e)
            }
        })?;

        if !meta.is_dir {
            return Err(AnalysisError::NotADirectory(path.display().to_string()));
        }

        // Get device ID for filesystem boundary checking
        self.root_dev = if self.config.no_cross {
            Some(get_device_id(path)?)
        } else {
            None
        };

        // Create root directory with base path
        let root_name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        let root = Arc::new(Dir::new_root(root_name, path.display().to_string()));
        root.set_size(meta.size);
        root.set_usage(meta.usage);
        root.set_mtime(meta.mtime);

        // Analyze contents with true parallelism
        self.analyze_dir_contents_parallel(path, root.clone())?;

        // Update stats
        root.update_stats();

        Ok(root)
    }

    /// Analyze directory contents with true parallel processing.
    fn analyze_dir_contents_parallel(&self, path: &Path, parent: DirRef) -> Result<(), AnalysisError> {
        // Read all entries first - this is the fastest way to get directory contents
        let entries: Vec<_> = match fs::read_dir(path) {
            Ok(entries) => entries.filter_map(|e| e.ok()).collect(),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    parent.set_flag(crate::fs::ItemFlag::PermissionDenied);
                    return Ok(());
                }
                return Err(AnalysisError::Io(e));
            }
        };

        // Pre-allocate with estimated capacity to reduce reallocations
        let estimated_count = entries.len();
        let mut file_infos: Vec<(std::path::PathBuf, FileMetadata, bool)> = Vec::with_capacity(estimated_count);

        // Collect file info in a single pass - use entry.metadata() for cached data
        for entry in entries {
            if self.should_ignore(&entry.path()) {
                continue;
            }
            let path = entry.path();
            // Use entry.metadata() which may be cached by the OS
            if let Ok(meta) = entry.metadata() {
                let file_meta = metadata_to_file_metadata(&meta);
                let is_dir = file_meta.is_dir;
                file_infos.push((path, file_meta, is_dir));
            }
        }

        // Separate files and directories - avoid allocation by reusing the vector
        let mut files: Vec<(std::path::PathBuf, FileMetadata, bool)> = Vec::with_capacity(estimated_count / 2);
        let mut dirs: Vec<(std::path::PathBuf, FileMetadata, bool)> = Vec::with_capacity(estimated_count / 2);

        for item in file_infos {
            if item.2 {
                dirs.push(item);
            } else {
                files.push(item);
            }
        }

        // Process files in parallel - use par_bridge for better performance with iterators
        let file_items: Vec<ItemRef> = files
            .par_iter()
            .filter_map(|(path, meta, _)| {
                self.process_file_with_meta(path, meta).ok()
            })
            .collect();

        // Batch add files to parent
        if !file_items.is_empty() {
            let mut parent_files = parent.files_write();
            parent_files.reserve(file_items.len());
            for item in file_items {
                parent_files.push(item);
            }
        }

        // Process directories in parallel using rayon's fork-join
        let parent_weak = Arc::downgrade(&parent);
        let dir_items: Vec<ItemRef> = dirs
            .par_iter()
            .filter_map(|(path, meta, _)| {
                self.process_directory_with_meta(path, meta, parent_weak.clone()).ok()
            })
            .collect();

        // Batch add directories to parent
        if !dir_items.is_empty() {
            let mut parent_files = parent.files_write();
            parent_files.reserve(dir_items.len());
            for item in dir_items {
                parent_files.push(item);
            }
        }

        Ok(())
    }

    /// Process a single file with pre-fetched metadata.
    fn process_file_with_meta(&self, path: &Path, meta: &FileMetadata) -> Result<ItemRef, AnalysisError> {
        // Skip special files
        if meta.is_special {
            let name = extract_filename(path);
            return Ok(create_file(name, &FileMetadata::default()));
        }

        // Track hard links (only on platforms that support it)
        if meta.nlink > 1 && meta.inode > 0 {
            let mut hl = self.hardlinks.lock();
            if hl.has_seen(meta.inode) {
                let name = extract_filename(path);
                return Ok(create_file(name, &FileMetadata {
                    size: 0,
                    usage: 0,
                    mtime: meta.mtime,
                    inode: meta.inode,
                    nlink: meta.nlink,
                    is_dir: false,
                    is_symlink: meta.is_symlink,
                    is_special: false,
                }));
            }
            hl.check_and_mark(meta.inode);
        }

        let name = extract_filename(path);

        // Only report progress if enabled (avoid overhead in non-interactive mode)
        if self.enable_progress {
            self.items_scanned.fetch_add(1, Ordering::Relaxed);
            self.progress.record_item(&path.display().to_string(), meta.size);
        }

        Ok(create_file(name, meta))
    }

    /// Process a directory with pre-fetched metadata.
    fn process_directory_with_meta(&self, path: &Path, meta: &FileMetadata, parent_weak: std::sync::Weak<Dir>) -> Result<ItemRef, AnalysisError> {
        // Check filesystem boundary
        if self.config.no_cross {
            if let Some(root_dev) = self.root_dev {
                if is_different_filesystem(path, root_dev)? {
                    let name = extract_filename(path);
                    let dir = create_dir(name, &FileMetadata::default());
                    dir.set_flag(crate::fs::ItemFlag::Error);
                    return Ok(dir);
                }
            }
        }

        let name = extract_filename(path);
        let dir = create_dir(name, meta);

        // Set parent reference for navigation
        if let Some(parent) = parent_weak.upgrade() {
            dir.set_parent(Arc::downgrade(&parent));
        }

        // Only report progress if enabled
        if self.enable_progress {
            self.items_scanned.fetch_add(1, Ordering::Relaxed);
            self.progress.record_item(&path.display().to_string(), meta.size);
        }

        // Recursively analyze with parallel processing
        self.analyze_dir_contents_parallel(path, dir.clone())?;
        dir.update_stats();

        Ok(dir)
    }

    /// Legacy method for compatibility.
    fn analyze_dir_contents(&self, path: &Path, parent: DirRef) -> Result<(), AnalysisError> {
        self.analyze_dir_contents_parallel(path, parent)
    }

    /// Process a single file (legacy, for compatibility).
    fn process_file(&self, path: &Path, _parent: DirRef) -> Result<ItemRef, AnalysisError> {
        let meta = get_metadata(path)?;
        self.process_file_with_meta(path, &meta)
    }

    /// Process a directory (legacy, for compatibility).
    fn process_directory(&self, path: &Path, parent: DirRef) -> Result<ItemRef, AnalysisError> {
        let meta = get_metadata(path)?;
        self.process_directory_with_meta(path, &meta, Arc::downgrade(&parent))
    }

    /// Check if a path should be ignored.
    fn should_ignore(&self, path: &Path) -> bool {
        self.config.ignore.should_ignore(path)
    }

    /// Get a progress reporter.
    pub fn progress(&self) -> Arc<ProgressReporter> {
        self.progress.clone()
    }
}

impl Default for ParallelAnalyzer {
    fn default() -> Self {
        Self::new(AnalyzerConfig::default())
    }
}

/// Extract filename from path efficiently.
#[inline]
fn extract_filename(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}
