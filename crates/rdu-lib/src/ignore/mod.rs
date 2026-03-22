//! Ignore pattern matching for directories and files.

use regex::Regex;
use std::path::Path;

/// Default directories to ignore on Unix systems.
#[cfg(unix)]
pub const DEFAULT_IGNORE_DIRS: &[&str] = &["/proc", "/dev", "/sys", "/run"];

/// Default directories to ignore on Windows.
#[cfg(windows)]
pub const DEFAULT_IGNORE_DIRS: &[&str] = &[];

/// Check if a directory should be ignored.
#[derive(Debug, Clone)]
pub struct IgnoreMatcher {
    /// Exact paths to ignore
    ignore_paths: Vec<String>,
    /// Regex patterns to ignore
    ignore_patterns: Vec<Regex>,
    /// Ignore hidden files/directories
    ignore_hidden: bool,
}

impl Default for IgnoreMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl IgnoreMatcher {
    /// Create a new ignore matcher with default settings.
    pub fn new() -> Self {
        Self {
            ignore_paths: DEFAULT_IGNORE_DIRS.iter().map(|s| s.to_string()).collect(),
            ignore_patterns: Vec::new(),
            ignore_hidden: false,
        }
    }

    /// Add paths to ignore.
    pub fn with_paths(mut self, paths: Vec<String>) -> Self {
        self.ignore_paths.extend(paths);
        self
    }

    /// Add regex patterns to ignore.
    pub fn with_patterns(mut self, patterns: Vec<String>) -> Result<Self, regex::Error> {
        for pattern in patterns {
            let re = Regex::new(&pattern)?;
            self.ignore_patterns.push(re);
        }
        Ok(self)
    }

    /// Set whether to ignore hidden files.
    pub fn with_ignore_hidden(mut self, ignore_hidden: bool) -> Self {
        self.ignore_hidden = ignore_hidden;
        self
    }

    /// Check if a path should be ignored.
    pub fn should_ignore(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check exact path matches
        for ignore_path in &self.ignore_paths {
            if path_str == *ignore_path || path_str.starts_with(&format!("{}/", ignore_path)) {
                return true;
            }
        }

        // Check regex patterns
        for pattern in &self.ignore_patterns {
            if pattern.is_match(&path_str) {
                return true;
            }
        }

        // Check hidden files
        if self.ignore_hidden {
            if let Some(name) = path.file_name() {
                if name.to_string_lossy().starts_with('.') {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a directory should be ignored.
    pub fn should_ignore_dir(&self, path: &Path) -> bool {
        self.should_ignore(path)
    }

    /// Check if a file should be ignored.
    pub fn should_ignore_file(&self, path: &Path) -> bool {
        // Only check hidden flag for files
        if self.ignore_hidden {
            if let Some(name) = path.file_name() {
                if name.to_string_lossy().starts_with('.') {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_ignore() {
        let matcher = IgnoreMatcher::new();
        #[cfg(unix)]
        assert!(matcher.should_ignore(Path::new("/proc")));
    }

    #[test]
    fn test_hidden_ignore() {
        let matcher = IgnoreMatcher::new().with_ignore_hidden(true);
        assert!(matcher.should_ignore(Path::new("/home/user/.cache")));
        assert!(!matcher.should_ignore(Path::new("/home/user/cache")));
    }

    #[test]
    fn test_pattern_ignore() {
        let matcher = IgnoreMatcher::new()
            .with_patterns(vec![r"^/tmp/.*".to_string()])
            .unwrap();
        assert!(matcher.should_ignore(Path::new("/tmp/something")));
        assert!(!matcher.should_ignore(Path::new("/home/user")));
    }
}
