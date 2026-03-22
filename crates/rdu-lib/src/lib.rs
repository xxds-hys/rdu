//! rdu-lib - Core library for disk usage analysis.
//!
//! This library provides functionality for analyzing disk usage
//! with support for parallel scanning, platform-specific metadata,
//! and various filtering options.

pub mod analyzer;
pub mod device;
pub mod export;
pub mod fs;
pub mod ignore;
pub mod platform;
pub mod timefilter;

// Re-export commonly used types
pub use analyzer::{
    AnalysisError, AnalysisResult, AnalyzerConfig, ParallelAnalyzer, Progress, ProgressReporter,
    SequentialAnalyzer,
};
pub use device::{get_devices, get_fs_stats, DeviceInfo, FsStats};
pub use export::{export_to_file, export_to_json, import_from_file, import_from_json, JsonItem};
pub use fs::{sort_items, Dir, DirRef, File, Item, ItemFlag, ItemRef, ItemType, SortBy, SortOrder};
pub use ignore::{IgnoreMatcher, DEFAULT_IGNORE_DIRS};
pub use platform::{get_metadata, metadata_to_file_metadata, FileMetadata};
pub use timefilter::{parse_date, parse_duration, ParseDateError, ParseDurationError, TimeFilter};
