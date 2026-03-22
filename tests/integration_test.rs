//! Integration tests for rdu library.

use rdu_lib::{
    export_to_json, import_from_json, sort_items, AnalyzerConfig, Dir, File, IgnoreMatcher,
    ParallelAnalyzer, SequentialAnalyzer, SortBy, SortOrder,
};
use std::sync::Arc;
use tempfile::TempDir;

/// Helper to create a test directory structure.
fn create_test_structure() -> TempDir {
    let temp = TempDir::new().expect("Failed to create temp dir");

    // Create some files
    std::fs::write(temp.path().join("file1.txt"), "a".repeat(100)).unwrap();
    std::fs::write(temp.path().join("file2.txt"), "b".repeat(200)).unwrap();

    // Create a subdirectory with files
    std::fs::create_dir(temp.path().join("subdir")).unwrap();
    std::fs::write(temp.path().join("subdir/file3.txt"), "c".repeat(300)).unwrap();
    std::fs::write(temp.path().join("subdir/file4.txt"), "d".repeat(400)).unwrap();

    // Create nested directory
    std::fs::create_dir(temp.path().join("subdir/nested")).unwrap();
    std::fs::write(temp.path().join("subdir/nested/file5.txt"), "e".repeat(500)).unwrap();

    // Create hidden directory (if not Windows)
    #[cfg(unix)]
    {
        std::fs::create_dir(temp.path().join(".hidden")).unwrap();
        std::fs::write(temp.path().join(".hidden/secret.txt"), "secret".repeat(50)).unwrap();
    }

    temp
}

#[test]
fn test_parallel_analyzer_basic() {
    let temp = create_test_structure();

    let config = AnalyzerConfig::default();
    let mut analyzer = ParallelAnalyzer::new(config);

    let result = analyzer.analyze(temp.path());
    assert!(result.is_ok());

    let root = result.unwrap();
    assert!(root.is_dir());
    assert!(root.item_count() > 0);
}

#[test]
fn test_sequential_analyzer_basic() {
    let temp = create_test_structure();

    let config = AnalyzerConfig::default();
    let mut analyzer = SequentialAnalyzer::new(config);

    let result = analyzer.analyze(temp.path());
    assert!(result.is_ok());

    let root = result.unwrap();
    assert!(root.is_dir());
    assert!(root.item_count() > 0);
}

#[test]
fn test_analyzer_detects_files() {
    let temp = create_test_structure();

    let config = AnalyzerConfig::default();
    let mut analyzer = ParallelAnalyzer::new(config);

    let root = analyzer.analyze(temp.path()).unwrap();
    let files = root.files_read();

    // Should have file1.txt, file2.txt, subdir
    assert!(files.len() >= 3);

    // Check that files are detected
    let file_names: Vec<&str> = files.iter().map(|f| f.name()).collect();
    assert!(file_names.contains(&"file1.txt"));
    assert!(file_names.contains(&"file2.txt"));
    assert!(file_names.contains(&"subdir"));
}

#[test]
fn test_analyzer_size_calculation() {
    let temp = create_test_structure();

    let config = AnalyzerConfig::default();
    let mut analyzer = ParallelAnalyzer::new(config);

    let root = analyzer.analyze(temp.path()).unwrap();

    // Total size should be at least 100+200+300+400+500 = 1500 bytes
    assert!(root.size() >= 1500);
}

#[test]
fn test_analyzer_with_ignore_patterns() {
    let temp = create_test_structure();

    let ignore = IgnoreMatcher::new()
        .with_paths(vec!["subdir".to_string()]);

    let config = AnalyzerConfig {
        ignore,
        ..Default::default()
    };

    let mut analyzer = ParallelAnalyzer::new(config);
    let root = analyzer.analyze(temp.path()).unwrap();
    let files = root.files_read();

    // subdir should be ignored
    let file_names: Vec<&str> = files.iter().map(|f| f.name()).collect();
    assert!(!file_names.contains(&"subdir"));
}

#[test]
fn test_analyzer_with_ignore_hidden() {
    let temp = create_test_structure();

    let ignore = IgnoreMatcher::new().with_ignore_hidden(true);

    let config = AnalyzerConfig {
        ignore,
        ..Default::default()
    };

    let mut analyzer = ParallelAnalyzer::new(config);
    let root = analyzer.analyze(temp.path()).unwrap();
    let files = root.files_read();

    // .hidden should be ignored
    let file_names: Vec<&str> = files.iter().map(|f| f.name()).collect();
    assert!(!file_names.contains(&".hidden"));
}

#[test]
fn test_analyzer_nonexistent_path() {
    let config = AnalyzerConfig::default();
    let mut analyzer = ParallelAnalyzer::new(config);

    let result = analyzer.analyze(std::path::Path::new("/nonexistent/path/12345"));
    assert!(result.is_err());
}

#[test]
fn test_analyzer_file_path() {
    let config = AnalyzerConfig::default();
    let mut analyzer = ParallelAnalyzer::new(config);

    // Analyzing a file should return an error
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("test.txt"), "hello").unwrap();

    let result = analyzer.analyze(temp.path().join("test.txt").as_path());
    assert!(result.is_err());
}

#[test]
fn test_export_import_consistency() {
    let temp = create_test_structure();

    let config = AnalyzerConfig::default();
    let mut analyzer = ParallelAnalyzer::new(config);

    let original = analyzer.analyze(temp.path()).unwrap();
    let original_size = original.size();
    let original_count = original.item_count();

    // Export to JSON
    let json = export_to_json(&original).unwrap();

    // Import back
    let imported = import_from_json(&json).unwrap();

    assert_eq!(imported.size(), original_size);
    assert_eq!(imported.item_count(), original_count);
}

#[test]
fn test_sort_items_by_size() {
    let mut items: Vec<rdu_lib::ItemRef> = vec![
        Arc::new(File::new("a".to_string()).with_size(100)),
        Arc::new(File::new("b".to_string()).with_size(300)),
        Arc::new(File::new("c".to_string()).with_size(200)),
    ];

    sort_items(&mut items, SortBy::Size, SortOrder::Desc);

    assert_eq!(items[0].name(), "b");
    assert_eq!(items[1].name(), "c");
    assert_eq!(items[2].name(), "a");
}

#[test]
fn test_sort_items_by_name() {
    let mut items: Vec<rdu_lib::ItemRef> = vec![
        Arc::new(File::new("charlie".to_string())),
        Arc::new(File::new("alice".to_string())),
        Arc::new(File::new("bob".to_string())),
    ];

    sort_items(&mut items, SortBy::Name, SortOrder::Asc);

    assert_eq!(items[0].name(), "alice");
    assert_eq!(items[1].name(), "bob");
    assert_eq!(items[2].name(), "charlie");
}

#[test]
fn test_empty_directory() {
    let temp = TempDir::new().unwrap();
    std::fs::create_dir(temp.path().join("empty")).unwrap();

    let config = AnalyzerConfig::default();
    let mut analyzer = ParallelAnalyzer::new(config);

    let root = analyzer.analyze(temp.path()).unwrap();
    let files = root.files_read();

    let empty_dir = files.iter().find(|f| f.name() == "empty").unwrap();
    assert!(empty_dir.is_dir());

    if let Some(dir) = empty_dir.as_any().downcast_ref::<Dir>() {
        assert!(dir.files_read().is_empty());
        assert_eq!(dir.item_count(), 0);
    }
}

#[test]
fn test_directory_structure() {
    let temp = create_test_structure();

    let config = AnalyzerConfig::default();
    let mut analyzer = ParallelAnalyzer::new(config);

    let root = analyzer.analyze(temp.path()).unwrap();

    // Find subdir
    let files = root.files_read();
    let subdir = files.iter().find(|f| f.name() == "subdir").unwrap();
    assert!(subdir.is_dir());

    // Check nested structure
    if let Some(dir) = subdir.as_any().downcast_ref::<Dir>() {
        let subfiles = dir.files_read();
        assert!(subfiles.iter().any(|f| f.name() == "nested"));
        assert!(subfiles.iter().any(|f| f.name() == "file3.txt"));
        assert!(subfiles.iter().any(|f| f.name() == "file4.txt"));
    }
}

#[test]
fn test_parallel_vs_sequential_consistency() {
    let temp = create_test_structure();

    // Parallel analysis
    let config = AnalyzerConfig::default();
    let mut parallel_analyzer = ParallelAnalyzer::new(config.clone());
    let parallel_result = parallel_analyzer.analyze(temp.path()).unwrap();

    // Sequential analysis
    let mut sequential_analyzer = SequentialAnalyzer::new(config);
    let sequential_result = sequential_analyzer.analyze(temp.path()).unwrap();

    // Both should find the same number of items
    assert_eq!(parallel_result.item_count(), sequential_result.item_count());
}

#[test]
fn test_ignore_regex_patterns() {
    let temp = create_test_structure();

    let ignore = IgnoreMatcher::new()
        .with_patterns(vec![r".*\.txt".to_string()])
        .unwrap();

    let config = AnalyzerConfig {
        ignore,
        ..Default::default()
    };

    let mut analyzer = ParallelAnalyzer::new(config);
    let root = analyzer.analyze(temp.path()).unwrap();
    let files = root.files_read();

    // All .txt files should be ignored
    for file in files.iter() {
        assert!(!file.name().ends_with(".txt"));
    }
}