//! Directory implementation.

use super::{Item, ItemFlag, ItemRef, ItemType, WeakDirRef};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::any::Any;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

/// A directory in the filesystem.
#[derive(Debug)]
pub struct Dir {
    /// Directory name
    name: String,
    /// Base path (for root directories)
    base_path: Option<String>,
    /// Apparent (logical) size in bytes (sum of all contained files)
    size: AtomicU64,
    /// Actual disk usage in bytes
    usage: AtomicU64,
    /// Modification time
    mtime: RwLock<SystemTime>,
    /// Flag indicating special state
    flag: RwLock<ItemFlag>,
    /// Multi-link inode for hard link detection
    inode: AtomicU64,
    /// Child items (files and subdirectories)
    files: RwLock<Vec<ItemRef>>,
    /// Total item count (recursive)
    item_count: AtomicUsize,
    /// Parent directory reference
    parent: RwLock<Option<WeakDirRef>>,
}

impl Dir {
    /// Create a new directory with the given name.
    pub fn new(name: String) -> Self {
        Self {
            name,
            base_path: None,
            size: AtomicU64::new(0),
            usage: AtomicU64::new(0),
            mtime: RwLock::new(SystemTime::UNIX_EPOCH),
            flag: RwLock::new(ItemFlag::Normal),
            inode: AtomicU64::new(0),
            files: RwLock::new(Vec::new()),
            item_count: AtomicUsize::new(0),
            parent: RwLock::new(None),
        }
    }

    /// Create a new root directory with a base path.
    pub fn new_root(name: String, base_path: String) -> Self {
        let mut dir = Self::new(name);
        dir.base_path = Some(base_path);
        dir
    }

    /// Get the base path (for root directories).
    pub fn base_path(&self) -> Option<&str> {
        self.base_path.as_deref()
    }

    /// Set the apparent size.
    pub fn set_size(&self, size: u64) {
        self.size.store(size, Ordering::Release);
    }

    /// Set the disk usage.
    pub fn set_usage(&self, usage: u64) {
        self.usage.store(usage, Ordering::Release);
    }

    /// Set the modification time.
    pub fn set_mtime(&self, mtime: SystemTime) {
        *self.mtime.write() = mtime;
    }

    /// Set the flag.
    pub fn set_flag(&self, flag: ItemFlag) {
        *self.flag.write() = flag;
    }

    /// Set the inode.
    pub fn set_inode(&self, inode: u64) {
        self.inode.store(inode, Ordering::Release);
    }

    /// Set the parent directory.
    pub fn set_parent(&self, parent: WeakDirRef) {
        *self.parent.write() = Some(parent);
    }

    /// Get the parent directory reference.
    pub fn parent(&self) -> Option<WeakDirRef> {
        self.parent.read().clone()
    }

    /// Add a child item.
    pub fn add_file(&self, item: ItemRef) {
        self.files.write().push(item);
    }

    /// Remove a child item by name.
    pub fn remove_file_by_name(&self, name: &str) {
        let mut files = self.files.write();
        files.retain(|f| f.name() != name);
    }

    /// Remove a specific child item.
    pub fn remove_file(&self, item: &ItemRef) {
        let mut files = self.files.write();
        if let Some(pos) = files.iter().position(|f| Arc::ptr_eq(f, item)) {
            files.remove(pos);
        }
    }

    /// Get a read lock on the files.
    pub fn files_read(&self) -> RwLockReadGuard<'_, Vec<ItemRef>> {
        self.files.read()
    }

    /// Get a write lock on the files.
    pub fn files_write(&self) -> RwLockWriteGuard<'_, Vec<ItemRef>> {
        self.files.write()
    }

    /// Add to the item count.
    pub fn add_item_count(&self, count: usize) {
        self.item_count.fetch_add(count, Ordering::Relaxed);
    }

    /// Set the item count.
    pub fn set_item_count(&self, count: usize) {
        self.item_count.store(count, Ordering::Release);
    }

    /// Update stats from children.
    pub fn update_stats(&self) {
        let files = self.files.read();
        let mut total_size = 0u64;
        let mut total_usage = 0u64;
        let mut total_count = 0usize;

        for item in files.iter() {
            total_size += item.size();
            total_usage += item.usage();
            total_count += if item.is_dir() {
                item.item_count() + 1
            } else {
                1
            };
        }

        drop(files);
        self.size.store(total_size, Ordering::Release);
        self.usage.store(total_usage, Ordering::Release);
        self.item_count.store(total_count, Ordering::Release);
    }

    /// Update stats excluding hard links.
    pub fn update_stats_exclude_hardlinks(&self, hardlinks: &std::collections::HashSet<u64>) {
        let files = self.files.read();
        let mut total_size = 0u64;
        let mut total_usage = 0u64;
        let mut total_count = 0usize;

        for item in files.iter() {
            let inode = item.multi_link_inode();
            if inode > 0 && hardlinks.contains(&inode) {
                continue;
            }
            total_size += item.size();
            total_usage += item.usage();
            total_count += if item.is_dir() {
                item.item_count() + 1
            } else {
                1
            };
        }

        drop(files);
        self.size.store(total_size, Ordering::Release);
        self.usage.store(total_usage, Ordering::Release);
        self.item_count.store(total_count, Ordering::Release);
    }
}

impl Item for Dir {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_dir(&self) -> bool {
        true
    }

    fn item_type(&self) -> ItemType {
        ItemType::Dir
    }

    fn size(&self) -> u64 {
        self.size.load(Ordering::Acquire)
    }

    fn usage(&self) -> u64 {
        self.usage.load(Ordering::Acquire)
    }

    fn mtime(&self) -> SystemTime {
        *self.mtime.read()
    }

    fn flag(&self) -> ItemFlag {
        *self.flag.read()
    }

    fn item_count(&self) -> usize {
        self.item_count.load(Ordering::Acquire)
    }

    fn path(&self) -> String {
        // Check parent first
        if let Some(parent_weak) = self.parent() {
            if let Some(parent) = parent_weak.upgrade() {
                return format!("{}/{}", parent.path(), self.name);
            }
        }
        // Fall back to base path
        if let Some(base) = &self.base_path {
            return base.clone();
        }
        self.name.clone()
    }

    fn multi_link_inode(&self) -> u64 {
        self.inode.load(Ordering::Acquire)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::{File, ItemRef};
    use std::sync::Arc;

    #[test]
    fn test_dir_new() {
        let dir = Dir::new("testdir".to_string());
        assert_eq!(dir.name(), "testdir");
        assert!(dir.is_dir());
        assert_eq!(dir.size(), 0);
        assert_eq!(dir.usage(), 0);
        assert_eq!(dir.item_count(), 0);
    }

    #[test]
    fn test_dir_new_root() {
        let dir = Dir::new_root("root".to_string(), "/path/to/root".to_string());
        assert_eq!(dir.base_path(), Some("/path/to/root"));
    }

    #[test]
    fn test_dir_setters() {
        let dir = Dir::new("test".to_string());
        let mtime = SystemTime::now();

        dir.set_size(1024);
        dir.set_usage(4096);
        dir.set_mtime(mtime);
        dir.set_flag(ItemFlag::Error);
        dir.set_inode(12345);
        dir.set_item_count(10);

        assert_eq!(dir.size(), 1024);
        assert_eq!(dir.usage(), 4096);
        assert_eq!(dir.mtime(), mtime);
        assert_eq!(dir.flag(), ItemFlag::Error);
        assert_eq!(dir.multi_link_inode(), 12345);
        assert_eq!(dir.item_count(), 10);
    }

    #[test]
    fn test_dir_add_file() {
        let dir = Arc::new(Dir::new("parent".to_string()));
        let file = Arc::new(File::new("child.txt".to_string()));

        dir.add_file(file.clone());
        let files = dir.files_read();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name(), "child.txt");
    }

    #[test]
    fn test_dir_remove_file() {
        let dir = Arc::new(Dir::new("parent".to_string()));
        let file: ItemRef = Arc::new(File::new("child.txt".to_string()));

        dir.add_file(file.clone());
        assert_eq!(dir.files_read().len(), 1);

        dir.remove_file(&file);
        assert_eq!(dir.files_read().len(), 0);
    }

    #[test]
    fn test_dir_remove_file_by_name() {
        let dir = Arc::new(Dir::new("parent".to_string()));
        dir.add_file(Arc::new(File::new("file1.txt".to_string())));
        dir.add_file(Arc::new(File::new("file2.txt".to_string())));

        dir.remove_file_by_name("file1.txt");
        let files = dir.files_read();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name(), "file2.txt");
    }

    #[test]
    fn test_dir_update_stats() {
        let dir = Arc::new(Dir::new("parent".to_string()));
        let file1 = Arc::new(File::new("file1.txt".to_string()).with_size(100).with_usage(100));
        let file2 = Arc::new(File::new("file2.txt".to_string()).with_size(200).with_usage(200));

        dir.add_file(file1);
        dir.add_file(file2);
        dir.update_stats();

        assert_eq!(dir.size(), 300);
        assert_eq!(dir.usage(), 300);
        assert_eq!(dir.item_count(), 2);
    }

    #[test]
    fn test_dir_item_count_with_subdir() {
        let parent = Arc::new(Dir::new("parent".to_string()));
        let child = Arc::new(Dir::new("child".to_string()));
        let file = Arc::new(File::new("file.txt".to_string()));

        child.add_file(file);
        child.update_stats();

        parent.add_file(child);
        parent.update_stats();

        // Parent should have: 1 subdir + 1 file in subdir = 2 items
        assert_eq!(parent.item_count(), 2);
    }

    #[test]
    fn test_dir_path() {
        let root = Arc::new(Dir::new_root("root".to_string(), "/home/user".to_string()));
        assert_eq!(root.path(), "/home/user");
    }

    #[test]
    fn test_dir_flags() {
        let dir = Dir::new("test".to_string());

        dir.set_flag(ItemFlag::Normal);
        assert_eq!(dir.flag(), ItemFlag::Normal);

        dir.set_flag(ItemFlag::Error);
        assert_eq!(dir.flag(), ItemFlag::Error);

        dir.set_flag(ItemFlag::PermissionDenied);
        assert_eq!(dir.flag(), ItemFlag::PermissionDenied);

        dir.set_flag(ItemFlag::Symlink);
        assert_eq!(dir.flag(), ItemFlag::Symlink);

        dir.set_flag(ItemFlag::HardLink);
        assert_eq!(dir.flag(), ItemFlag::HardLink);
    }
}
