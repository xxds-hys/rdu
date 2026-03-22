//! File implementation.

use super::{Item, ItemFlag, ItemType};
use std::any::Any;
use std::sync::{Arc, Weak};
use std::time::SystemTime;

/// A regular file in the filesystem.
#[derive(Debug)]
pub struct File {
    /// File name
    name: String,
    /// Apparent (logical) size in bytes
    size: u64,
    /// Actual disk usage in bytes (blocks * 512 on Unix)
    usage: u64,
    /// Modification time
    mtime: SystemTime,
    /// Flag indicating special state
    flag: ItemFlag,
    /// Multi-link inode for hard link detection
    inode: u64,
    /// Parent directory reference
    parent: Option<Weak<super::Dir>>,
}

impl File {
    /// Create a new file with the given name.
    pub fn new(name: String) -> Self {
        Self {
            name,
            size: 0,
            usage: 0,
            mtime: SystemTime::UNIX_EPOCH,
            flag: ItemFlag::Normal,
            inode: 0,
            parent: None,
        }
    }

    /// Set the apparent size.
    pub fn with_size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    /// Set the disk usage.
    pub fn with_usage(mut self, usage: u64) -> Self {
        self.usage = usage;
        self
    }

    /// Set the modification time.
    pub fn with_mtime(mut self, mtime: SystemTime) -> Self {
        self.mtime = mtime;
        self
    }

    /// Set the flag.
    pub fn with_flag(mut self, flag: ItemFlag) -> Self {
        self.flag = flag;
        self
    }

    /// Set the inode.
    pub fn with_inode(mut self, inode: u64) -> Self {
        self.inode = inode;
        self
    }

    /// Set the parent directory.
    pub fn set_parent(&mut self, parent: Weak<super::Dir>) {
        self.parent = Some(parent);
    }

    /// Get the parent directory reference.
    pub fn parent(&self) -> Option<&Weak<super::Dir>> {
        self.parent.as_ref()
    }
}

impl Item for File {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_dir(&self) -> bool {
        false
    }

    fn item_type(&self) -> ItemType {
        ItemType::File
    }

    fn size(&self) -> u64 {
        self.size
    }

    fn usage(&self) -> u64 {
        self.usage
    }

    fn mtime(&self) -> SystemTime {
        self.mtime
    }

    fn flag(&self) -> ItemFlag {
        self.flag
    }

    fn item_count(&self) -> usize {
        0
    }

    fn path(&self) -> String {
        if let Some(parent) = &self.parent {
            if let Some(dir) = parent.upgrade() {
                return format!("{}/{}", dir.path(), self.name);
            }
        }
        self.name.clone()
    }

    fn multi_link_inode(&self) -> u64 {
        self.inode
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_file_new() {
        let file = File::new("test.txt".to_string());
        assert_eq!(file.name(), "test.txt");
        assert!(!file.is_dir());
        assert_eq!(file.item_type(), ItemType::File);
        assert_eq!(file.size(), 0);
        assert_eq!(file.usage(), 0);
        assert_eq!(file.item_count(), 0);
    }

    #[test]
    fn test_file_with_size() {
        let file = File::new("test.txt".to_string()).with_size(1024);
        assert_eq!(file.size(), 1024);
    }

    #[test]
    fn test_file_with_usage() {
        let file = File::new("test.txt".to_string()).with_usage(4096);
        assert_eq!(file.usage(), 4096);
    }

    #[test]
    fn test_file_with_mtime() {
        let mtime = SystemTime::now();
        let file = File::new("test.txt".to_string()).with_mtime(mtime);
        assert_eq!(file.mtime(), mtime);
    }

    #[test]
    fn test_file_with_flag() {
        let file = File::new("link".to_string()).with_flag(ItemFlag::Symlink);
        assert_eq!(file.flag(), ItemFlag::Symlink);
    }

    #[test]
    fn test_file_with_inode() {
        let file = File::new("test.txt".to_string()).with_inode(12345);
        assert_eq!(file.multi_link_inode(), 12345);
    }

    #[test]
    fn test_file_builder_pattern() {
        let mtime = SystemTime::now() - Duration::from_secs(3600);
        let file = File::new("test.txt".to_string())
            .with_size(2048)
            .with_usage(4096)
            .with_mtime(mtime)
            .with_flag(ItemFlag::HardLink)
            .with_inode(999);

        assert_eq!(file.name(), "test.txt");
        assert_eq!(file.size(), 2048);
        assert_eq!(file.usage(), 4096);
        assert_eq!(file.mtime(), mtime);
        assert_eq!(file.flag(), ItemFlag::HardLink);
        assert_eq!(file.multi_link_inode(), 999);
    }

    #[test]
    fn test_file_path_without_parent() {
        let file = File::new("test.txt".to_string());
        assert_eq!(file.path(), "test.txt");
    }

    #[test]
    fn test_file_all_flags() {
        let flags = vec![
            ItemFlag::Normal,
            ItemFlag::Error,
            ItemFlag::Symlink,
            ItemFlag::HardLink,
            ItemFlag::PermissionDenied,
        ];

        for flag in flags {
            let file = File::new("test.txt".to_string()).with_flag(flag);
            assert_eq!(file.flag(), flag);
        }
    }
}
