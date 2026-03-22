//! Filesystem data structures for disk usage analysis.

mod dir;
mod file;
mod item;
mod sorting;

pub use dir::Dir;
pub use file::File;
pub use item::{Item, ItemFlag, ItemType};
pub use sorting::{sort_items, SortBy, SortOrder};

use std::sync::{Arc, Weak};

/// Type alias for a reference-counted item.
pub type ItemRef = Arc<dyn Item>;

/// Type alias for a weak reference to a directory.
pub type WeakDirRef = Weak<Dir>;

/// Type alias for a reference-counted directory.
pub type DirRef = Arc<Dir>;
