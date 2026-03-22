//! Import/export functionality for disk usage data.

use crate::fs::{Dir, DirRef, File, ItemFlag, ItemRef};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// JSON representation of a filesystem item.
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonItem {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtime: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag: Option<char>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inode: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<JsonItem>>,
}

impl JsonItem {
    /// Convert a filesystem item to JSON format.
    pub fn from_item(item: &ItemRef) -> Self {
        let mtime = item
            .mtime()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .ok();

        let flag = match item.flag() {
            ItemFlag::Normal => None,
            ItemFlag::Error => Some('!'),
            ItemFlag::Symlink => Some('@'),
            ItemFlag::HardLink => Some('H'),
            ItemFlag::PermissionDenied => Some('.'),
        };

        if item.is_dir() {
            let files: Vec<JsonItem> = if let Some(dir_ref) = item.as_any().downcast_ref::<Dir>() {
                dir_ref
                    .files_read()
                    .iter()
                    .map(|f| Self::from_item(f))
                    .collect()
            } else {
                Vec::new()
            };

            Self {
                name: item.name().to_string(),
                size: Some(item.size()),
                usage: Some(item.usage()),
                mtime,
                flag,
                inode: if item.multi_link_inode() > 0 {
                    Some(item.multi_link_inode())
                } else {
                    None
                },
                item_count: Some(item.item_count()),
                files: if files.is_empty() { None } else { Some(files) },
            }
        } else {
            Self {
                name: item.name().to_string(),
                size: Some(item.size()),
                usage: Some(item.usage()),
                mtime,
                flag,
                inode: if item.multi_link_inode() > 0 {
                    Some(item.multi_link_inode())
                } else {
                    None
                },
                item_count: None,
                files: None,
            }
        }
    }

    /// Convert JSON item to a filesystem item.
    pub fn to_item(&self) -> ItemRef {
        if let Some(files) = &self.files {
            // This is a directory
            let dir = Arc::new(Dir::new(self.name.clone()));

            if let Some(size) = self.size {
                dir.set_size(size);
            }
            if let Some(usage) = self.usage {
                dir.set_usage(usage);
            }
            if let Some(mtime) = self.mtime {
                dir.set_mtime(
                    SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(mtime as u64),
                );
            }
            if let Some(flag) = self.flag {
                dir.set_flag(match flag {
                    '!' => ItemFlag::Error,
                    '@' => ItemFlag::Symlink,
                    'H' => ItemFlag::HardLink,
                    '.' => ItemFlag::PermissionDenied,
                    _ => ItemFlag::Normal,
                });
            }
            if let Some(inode) = self.inode {
                dir.set_inode(inode);
            }
            if let Some(count) = self.item_count {
                dir.set_item_count(count);
            }

            for json_file in files {
                let child = json_file.to_item();
                dir.add_file(child);
            }

            dir
        } else {
            // This is a file
            let mut file = File::new(self.name.clone());

            if let Some(size) = self.size {
                file = file.with_size(size);
            }
            if let Some(usage) = self.usage {
                file = file.with_usage(usage);
            }
            if let Some(mtime) = self.mtime {
                file = file.with_mtime(
                    SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(mtime as u64),
                );
            }
            if let Some(flag) = self.flag {
                file = file.with_flag(match flag {
                    '!' => ItemFlag::Error,
                    '@' => ItemFlag::Symlink,
                    'H' => ItemFlag::HardLink,
                    '.' => ItemFlag::PermissionDenied,
                    _ => ItemFlag::Normal,
                });
            }
            if let Some(inode) = self.inode {
                file = file.with_inode(inode);
            }

            Arc::new(file)
        }
    }
}

/// Export a directory to JSON.
pub fn export_to_json(dir: &ItemRef) -> Result<String, serde_json::Error> {
    let json_item = JsonItem::from_item(dir);
    serde_json::to_string_pretty(&json_item)
}

/// Export a directory to JSON file.
pub fn export_to_file(dir: &ItemRef, path: &Path) -> std::io::Result<()> {
    let json =
        export_to_json(dir).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, json)
}

/// Import a directory from JSON.
pub fn import_from_json(json: &str) -> Result<ItemRef, serde_json::Error> {
    let json_item: JsonItem = serde_json::from_str(json)?;
    Ok(json_item.to_item())
}

/// Import a directory from JSON file.
pub fn import_from_file(path: &Path) -> std::io::Result<ItemRef> {
    let json = std::fs::read_to_string(path)?;
    import_from_json(&json).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::DirRef;
    use std::time::{Duration, SystemTime};

    fn create_test_file(name: &str, size: u64) -> ItemRef {
        Arc::new(
            File::new(name.to_string())
                .with_size(size)
                .with_usage(size),
        )
    }

    fn create_test_dir(name: &str) -> DirRef {
        Arc::new(Dir::new(name.to_string()))
    }

    #[test]
    fn test_json_item_from_file() {
        let file = create_test_file("test.txt", 1024);
        let json = JsonItem::from_item(&file);

        assert_eq!(json.name, "test.txt");
        assert_eq!(json.size, Some(1024));
        assert_eq!(json.usage, Some(1024));
        assert!(json.files.is_none());
    }

    #[test]
    fn test_json_item_from_dir() {
        let dir: ItemRef = create_test_dir("mydir");
        let file = create_test_file("child.txt", 100);
        if let Some(dir_ref) = dir.as_any().downcast_ref::<Dir>() {
            dir_ref.add_file(file);
            dir_ref.update_stats();
        }

        let json = JsonItem::from_item(&dir);

        assert_eq!(json.name, "mydir");
        assert!(json.files.is_some());
        let files = json.files.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "child.txt");
    }

    #[test]
    fn test_json_item_with_mtime() {
        let mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(1609459200); // 2021-01-01
        let file: ItemRef = Arc::new(File::new("test.txt".to_string()).with_mtime(mtime));

        let json = JsonItem::from_item(&file);
        assert_eq!(json.mtime, Some(1609459200));
    }

    #[test]
    fn test_json_item_with_flag() {
        let file: ItemRef = Arc::new(
            File::new("link".to_string()).with_flag(ItemFlag::Symlink),
        );

        let json = JsonItem::from_item(&file);
        assert_eq!(json.flag, Some('@'));
    }

    #[test]
    fn test_json_item_flag_chars() {
        let test_cases = vec![
            (ItemFlag::Error, '!'),
            (ItemFlag::Symlink, '@'),
            (ItemFlag::HardLink, 'H'),
            (ItemFlag::PermissionDenied, '.'),
        ];

        for (flag, expected_char) in test_cases {
            let file: ItemRef = Arc::new(File::new("test".to_string()).with_flag(flag));
            let json = JsonItem::from_item(&file);
            assert_eq!(json.flag, Some(expected_char));
        }
    }

    #[test]
    fn test_export_import_roundtrip() {
        let dir: ItemRef = create_test_dir("root");
        if let Some(dir_ref) = dir.as_any().downcast_ref::<Dir>() {
            dir_ref.add_file(create_test_file("file1.txt", 100));
            dir_ref.add_file(create_test_file("file2.txt", 200));

            let subdir = create_test_dir("subdir");
            subdir.add_file(create_test_file("file3.txt", 300));
            subdir.update_stats();
            dir_ref.add_file(subdir);
            dir_ref.update_stats();
        }

        // Export to JSON
        let json = export_to_json(&dir).unwrap();

        // Import back
        let imported = import_from_json(&json).unwrap();

        assert_eq!(imported.name(), "root");
        assert!(imported.is_dir());
        assert_eq!(imported.size(), 600);
    }

    #[test]
    fn test_json_item_to_file() {
        let json = JsonItem {
            name: "test.txt".to_string(),
            size: Some(1024),
            usage: Some(4096),
            mtime: Some(1609459200),
            flag: None,
            inode: Some(12345),
            item_count: None,
            files: None,
        };

        let item = json.to_item();
        assert!(!item.is_dir());
        assert_eq!(item.name(), "test.txt");
        assert_eq!(item.size(), 1024);
        assert_eq!(item.usage(), 4096);
        assert_eq!(item.multi_link_inode(), 12345);
    }

    #[test]
    fn test_json_item_to_dir() {
        let json = JsonItem {
            name: "mydir".to_string(),
            size: Some(1000),
            usage: Some(4096),
            mtime: None,
            flag: Some('!'),
            inode: None,
            item_count: Some(10),
            files: Some(vec![JsonItem {
                name: "child.txt".to_string(),
                size: Some(100),
                usage: Some(100),
                mtime: None,
                flag: None,
                inode: None,
                item_count: None,
                files: None,
            }]),
        };

        let item = json.to_item();
        assert!(item.is_dir());
        assert_eq!(item.name(), "mydir");
        assert_eq!(item.size(), 1000);
        assert_eq!(item.item_count(), 10);
        assert_eq!(item.flag(), ItemFlag::Error);
    }

    #[test]
    fn test_export_to_json_empty_dir() {
        let dir: ItemRef = create_test_dir("empty");
        let json = export_to_json(&dir).unwrap();

        assert!(json.contains("\"name\": \"empty\""));
        assert!(!json.contains("\"files\"")); // files should be omitted when empty
    }

    #[test]
    fn test_import_invalid_json() {
        let result = import_from_json("not valid json");
        assert!(result.is_err());
    }
}
