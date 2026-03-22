//! Sorting functionality for filesystem items.

use super::{Item, ItemRef};
use std::cmp::Ordering;

/// Criteria for sorting items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortBy {
    /// Sort by disk usage (default)
    #[default]
    Usage,
    /// Sort by apparent size
    Size,
    /// Sort by name
    Name,
    /// Sort by item count (for directories)
    ItemCount,
    /// Sort by modification time
    Mtime,
}

/// Sort order direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    /// Descending (largest first)
    #[default]
    Desc,
    /// Ascending (smallest first)
    Asc,
}

impl SortBy {
    /// Get the next sort criteria.
    pub fn next(&self) -> Self {
        match self {
            SortBy::Usage => SortBy::Size,
            SortBy::Size => SortBy::Name,
            SortBy::Name => SortBy::ItemCount,
            SortBy::ItemCount => SortBy::Mtime,
            SortBy::Mtime => SortBy::Usage,
        }
    }

    /// Get a display name for this sort criteria.
    pub fn display_name(&self) -> &'static str {
        match self {
            SortBy::Usage => "disk usage",
            SortBy::Size => "size",
            SortBy::Name => "name",
            SortBy::ItemCount => "item count",
            SortBy::Mtime => "modification time",
        }
    }
}

impl SortOrder {
    /// Toggle the sort order.
    pub fn toggle(&self) -> Self {
        match self {
            SortOrder::Desc => SortOrder::Asc,
            SortOrder::Asc => SortOrder::Desc,
        }
    }

    /// Get a display name for this sort order.
    pub fn display_name(&self) -> &'static str {
        match self {
            SortOrder::Desc => "descending",
            SortOrder::Asc => "ascending",
        }
    }
}

/// Compare two items by the given sort criteria.
/// Returns ordering in descending order by default (b compared to a).
fn compare_items(a: &ItemRef, b: &ItemRef, sort_by: SortBy) -> Ordering {
    match sort_by {
        SortBy::Usage => b.usage().cmp(&a.usage()),
        SortBy::Size => b.size().cmp(&a.size()),
        SortBy::Name => b.name().cmp(a.name()),
        SortBy::ItemCount => b.item_count().cmp(&a.item_count()),
        SortBy::Mtime => b.mtime().cmp(&a.mtime()),
    }
}

/// Compare two items with directories always first.
fn compare_with_dirs_first(a: &ItemRef, b: &ItemRef, sort_by: SortBy) -> Ordering {
    match (a.is_dir(), b.is_dir()) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => compare_items(a, b, sort_by),
    }
}

/// Sort a list of items in place.
pub fn sort_items(items: &mut [ItemRef], sort_by: SortBy, order: SortOrder) {
    sort_items_with_dirs_first(items, sort_by, order, true);
}

/// Sort a list of items with an option to group directories first.
pub fn sort_items_with_dirs_first(
    items: &mut [ItemRef],
    sort_by: SortBy,
    order: SortOrder,
    dirs_first: bool,
) {
    items.sort_by(|a, b| {
        let cmp = if dirs_first {
            compare_with_dirs_first(a, b, sort_by)
        } else {
            compare_items(a, b, sort_by)
        };

        match order {
            SortOrder::Desc => cmp,
            SortOrder::Asc => cmp.reverse(),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::File;
    use std::sync::Arc;
    use std::time::SystemTime;

    fn make_file(name: &str, size: u64) -> ItemRef {
        Arc::new(File::new(name.to_string()).with_size(size).with_usage(size))
    }

    #[test]
    fn test_sort_by_usage() {
        let mut items: Vec<ItemRef> = vec![
            make_file("a", 100),
            make_file("b", 300),
            make_file("c", 200),
        ];

        sort_items(&mut items, SortBy::Usage, SortOrder::Desc);
        assert_eq!(items[0].name(), "b");
        assert_eq!(items[1].name(), "c");
        assert_eq!(items[2].name(), "a");
    }

    #[test]
    fn test_sort_by_name() {
        let mut items: Vec<ItemRef> = vec![
            make_file("c", 100),
            make_file("a", 300),
            make_file("b", 200),
        ];

        sort_items(&mut items, SortBy::Name, SortOrder::Asc);
        assert_eq!(items[0].name(), "a");
        assert_eq!(items[1].name(), "b");
        assert_eq!(items[2].name(), "c");
    }
}
