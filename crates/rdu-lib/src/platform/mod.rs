//! Platform-specific filesystem metadata extraction.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod other;
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub use other::*;

use std::path::Path;
use std::time::SystemTime;

/// File metadata with platform-specific information.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// Apparent (logical) size in bytes
    pub size: u64,
    /// Actual disk usage in bytes
    pub usage: u64,
    /// Modification time
    pub mtime: SystemTime,
    /// Inode number (for hard link detection)
    pub inode: u64,
    /// Number of hard links
    pub nlink: u64,
    /// Is this a directory
    pub is_dir: bool,
    /// Is this a symlink
    pub is_symlink: bool,
    /// Is this a special file (socket, fifo, device)
    pub is_special: bool,
}

impl Default for FileMetadata {
    fn default() -> Self {
        Self {
            size: 0,
            usage: 0,
            mtime: SystemTime::UNIX_EPOCH,
            inode: 0,
            nlink: 1,
            is_dir: false,
            is_symlink: false,
            is_special: false,
        }
    }
}

/// Get metadata for a file or directory.
pub fn get_metadata(path: &Path) -> std::io::Result<FileMetadata> {
    get_platform_metadata(path)
}

/// Convert std::fs::Metadata to FileMetadata (avoids extra syscalls).
pub fn metadata_to_file_metadata(meta: &std::fs::Metadata) -> FileMetadata {
    metadata_to_file_metadata_platform(meta)
}
