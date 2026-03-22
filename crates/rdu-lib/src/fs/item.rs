//! Item trait and related types.

use std::any::Any;
use std::sync::Arc;
use std::time::SystemTime;

/// Flag indicating special file states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemFlag {
    /// Normal file/directory
    Normal,
    /// Error reading this item
    Error,
    /// Symbolic link or socket
    Symlink,
    /// Hard link
    HardLink,
    /// Permission denied
    PermissionDenied,
}

impl Default for ItemFlag {
    fn default() -> Self {
        Self::Normal
    }
}

impl ItemFlag {
    /// Get the display character for this flag.
    pub fn as_char(&self) -> char {
        match self {
            ItemFlag::Normal => ' ',
            ItemFlag::Error => '!',
            ItemFlag::Symlink => '@',
            ItemFlag::HardLink => 'H',
            ItemFlag::PermissionDenied => '.',
        }
    }
}

/// Type of filesystem item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemType {
    /// Regular file
    File,
    /// Directory
    Dir,
}

/// Trait representing a filesystem item (file or directory).
///
/// This trait provides a common interface for both files and directories,
/// allowing uniform handling during analysis and display.
pub trait Item: Send + Sync {
    /// Get the name of this item.
    fn name(&self) -> &str;

    /// Check if this item is a directory.
    fn is_dir(&self) -> bool;

    /// Get the type of this item.
    fn item_type(&self) -> ItemType;

    /// Get the apparent (logical) size in bytes.
    fn size(&self) -> u64;

    /// Get the actual disk usage in bytes.
    /// On Unix, this is blocks * 512.
    /// On Windows, this equals apparent size.
    fn usage(&self) -> u64;

    /// Get the modification time.
    fn mtime(&self) -> SystemTime;

    /// Get the flag indicating special state.
    fn flag(&self) -> ItemFlag;

    /// Get the total item count (files + directories) for directories.
    /// Returns 0 for files.
    fn item_count(&self) -> usize;

    /// Get the full path of this item.
    fn path(&self) -> String;

    /// Get the multi-link inode number (for hard link detection).
    /// Returns 0 if not a hard link.
    fn multi_link_inode(&self) -> u64;

    /// Check if this item has an error.
    fn has_error(&self) -> bool {
        matches!(self.flag(), ItemFlag::Error | ItemFlag::PermissionDenied)
    }

    /// Get a display name with trailing slash for directories.
    fn display_name(&self) -> String {
        if self.is_dir() {
            format!("{}/", self.name())
        } else {
            self.name().to_string()
        }
    }

    /// Get a reference to this item as Any for downcasting.
    fn as_any(&self) -> &dyn Any;
}

impl std::fmt::Debug for dyn Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Item")
            .field("name", &self.name())
            .field("is_dir", &self.is_dir())
            .field("size", &self.size())
            .field("usage", &self.usage())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_flag_default() {
        let flag: ItemFlag = Default::default();
        assert_eq!(flag, ItemFlag::Normal);
    }

    #[test]
    fn test_item_flag_as_char() {
        assert_eq!(ItemFlag::Normal.as_char(), ' ');
        assert_eq!(ItemFlag::Error.as_char(), '!');
        assert_eq!(ItemFlag::Symlink.as_char(), '@');
        assert_eq!(ItemFlag::HardLink.as_char(), 'H');
        assert_eq!(ItemFlag::PermissionDenied.as_char(), '.');
    }

    #[test]
    fn test_item_type_equality() {
        assert_eq!(ItemType::File, ItemType::File);
        assert_eq!(ItemType::Dir, ItemType::Dir);
        assert_ne!(ItemType::File, ItemType::Dir);
    }

    #[test]
    fn test_item_flag_equality() {
        assert_eq!(ItemFlag::Normal, ItemFlag::Normal);
        assert_ne!(ItemFlag::Normal, ItemFlag::Error);
        assert_ne!(ItemFlag::Symlink, ItemFlag::HardLink);
    }
}
