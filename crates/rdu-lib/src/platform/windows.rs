//! Windows-specific filesystem metadata extraction.

use super::FileMetadata;
use std::fs;
use std::os::windows::fs::MetadataExt;
use std::path::Path;
use std::time::SystemTime;

pub fn get_platform_metadata(path: &Path) -> std::io::Result<FileMetadata> {
    let meta = fs::metadata(path)?;
    Ok(metadata_to_file_metadata_platform(&meta))
}

/// Convert std::fs::Metadata to FileMetadata (avoids extra syscalls).
pub fn metadata_to_file_metadata_platform(meta: &std::fs::Metadata) -> FileMetadata {
    // On Windows, we don't have block information, so usage = size
    let size = meta.len();
    let usage = size; // No block info on Windows

    let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let is_dir = meta.is_dir();
    let is_symlink = meta.file_type().is_symlink();

    // Windows doesn't have the same special file types
    let is_special = false;

    FileMetadata {
        size,
        usage,
        mtime,
        inode: 0, // Will be 0 on Windows
        nlink: 1, // Windows doesn't have hard links in the same way
        is_dir,
        is_symlink,
        is_special,
    }
}

/// Check if a path is on a different filesystem.
/// On Windows, we always return false (cross-filesystem is allowed).
pub fn is_different_filesystem(_path: &Path, _root_dev: u64) -> std::io::Result<bool> {
    Ok(false) // Windows doesn't have the same filesystem boundary concept
}

/// Get the device ID for a path.
/// On Windows, returns 0 (no device ID concept).
pub fn get_device_id(_path: &Path) -> std::io::Result<u64> {
    Ok(0)
}
