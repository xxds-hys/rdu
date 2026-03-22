//! Generic filesystem metadata extraction for other platforms (macOS, BSD, etc.).

use super::FileMetadata;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::time::SystemTime;

pub fn get_platform_metadata(path: &Path) -> std::io::Result<FileMetadata> {
    let meta = fs::metadata(path)?;
    Ok(metadata_to_file_metadata_platform(&meta))
}

/// Convert std::fs::Metadata to FileMetadata (avoids extra syscalls).
pub fn metadata_to_file_metadata_platform(meta: &std::fs::Metadata) -> FileMetadata {
    // On Unix-like systems, disk usage is blocks * 512
    let usage = meta.blocks() * 512;
    let size = meta.len();
    let inode = meta.ino();
    let nlink = meta.nlink() as u64;
    let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let is_dir = meta.is_dir();
    let is_symlink = meta.file_type().is_symlink();

    // Check for special files (Unix-like)
    let mode = meta.mode();
    let is_special = (mode & 0o170000) == 0o140000  // Socket
        || (mode & 0o170000) == 0o010000  // FIFO
        || (mode & 0o170000) == 0o020000  // Character device
        || (mode & 0o170000) == 0o060000; // Block device

    FileMetadata {
        size,
        usage,
        mtime,
        inode,
        nlink,
        is_dir,
        is_symlink,
        is_special,
    }
}

/// Check if a path is on a different filesystem.
pub fn is_different_filesystem(path: &Path, root_dev: u64) -> std::io::Result<bool> {
    let meta = fs::metadata(path)?;
    Ok(meta.dev() != root_dev)
}

/// Get the device ID for a path.
pub fn get_device_id(path: &Path) -> std::io::Result<u64> {
    let meta = fs::metadata(path)?;
    Ok(meta.dev())
}
