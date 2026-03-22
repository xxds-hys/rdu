//! Sequential disk usage analyzer.

use super::{
    create_dir, create_file, AnalysisError, AnalysisResult, AnalyzerConfig, HardLinkTracker,
    ProgressReporter,
};
use crate::fs::{DirRef, ItemRef};
use crate::platform::{get_device_id, get_metadata, is_different_filesystem, FileMetadata};
use parking_lot::Mutex;
use std::fs;
use std::path::Path;
use std::sync::Arc;

/// Sequential disk usage analyzer.
///
/// Use this for HDDs or when parallel overhead is not desired.
pub struct SequentialAnalyzer {
    config: AnalyzerConfig,
    progress: Arc<ProgressReporter>,
    hardlinks: Mutex<HardLinkTracker>,
    root_dev: Option<u64>,
}

impl SequentialAnalyzer {
    /// Create a new sequential analyzer.
    pub fn new(config: AnalyzerConfig) -> Self {
        Self {
            config,
            progress: Arc::new(ProgressReporter::new()),
            hardlinks: Mutex::new(HardLinkTracker::new()),
            root_dev: None,
        }
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

        // Create root directory
        let root = create_dir(
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string()),
            &meta,
        );

        // Analyze contents recursively
        self.analyze_dir_contents(path, root.clone())?;

        // Update stats
        root.update_stats();

        Ok(root)
    }

    /// Analyze the contents of a directory.
    fn analyze_dir_contents(&self, path: &Path, parent: DirRef) -> Result<(), AnalysisError> {
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    parent.set_flag(crate::fs::ItemFlag::PermissionDenied);
                    return Ok(());
                }
                return Err(AnalysisError::Io(e));
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let entry_path = entry.path();

            if self.should_ignore(&entry_path) {
                continue;
            }

            let meta = match get_metadata(&entry_path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            if meta.is_dir {
                // Process directory recursively
                if let Ok(item) = self.process_directory(&entry_path, meta, parent.clone()) {
                    parent.add_file(item);
                }
            } else {
                // Process file
                if let Ok(item) = self.process_file(&entry_path, meta) {
                    parent.add_file(item);
                }
            }
        }

        Ok(())
    }

    /// Process a single file.
    fn process_file(&self, path: &Path, mut meta: FileMetadata) -> Result<ItemRef, AnalysisError> {
        // Skip special files
        if meta.is_special {
            return Ok(create_file(
                path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                &FileMetadata::default(),
            ));
        }

        // Track hard links
        if meta.nlink > 1 {
            let mut hl = self.hardlinks.lock();
            if hl.has_seen(meta.inode) {
                meta.size = 0;
                meta.usage = 0;
            } else {
                hl.check_and_mark(meta.inode);
            }
        }

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        self.progress
            .record_item(&path.display().to_string(), meta.size);

        Ok(create_file(name, &meta))
    }

    /// Process a directory.
    fn process_directory(
        &self,
        path: &Path,
        meta: FileMetadata,
        _parent: DirRef,
    ) -> Result<ItemRef, AnalysisError> {
        // Check filesystem boundary
        if self.config.no_cross {
            if let Some(root_dev) = self.root_dev {
                if is_different_filesystem(path, root_dev)? {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let dir = create_dir(name, &FileMetadata::default());
                    dir.set_flag(crate::fs::ItemFlag::Error);
                    return Ok(dir);
                }
            }
        }

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let dir = create_dir(name, &meta);

        self.progress
            .record_item(&path.display().to_string(), meta.size);

        // Recursively analyze
        self.analyze_dir_contents(path, dir.clone())?;
        dir.update_stats();

        Ok(dir)
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

impl Default for SequentialAnalyzer {
    fn default() -> Self {
        Self::new(AnalyzerConfig::default())
    }
}
