# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2024-03-19

### Added

- Initial release of rdu (Rust Disk Usage)
- Core library (`rdu-lib`) with parallel and sequential analyzers
- Terminal UI (`rdu-tui`) for interactive disk usage browsing
- CLI application (`rdu-cli`) with full command-line interface
- Cross-platform support (Linux, Windows, macOS)
- Parallel directory scanning using Rayon
- Hard link detection and proper counting
- File system boundary checking (`-x` flag)
- JSON import/export functionality
- Multiple sorting options (size, name, count, mtime)
- Ignore patterns and hidden file filtering
- Time filtering (since, until, max-age, min-age)
- Comprehensive unit and integration tests

### Features Compared to gdu

| Feature | gdu (Go) | rdu (Rust) |
|---------|----------|------------|
| Parallel scanning | ✅ | ✅ |
| Sequential scanning | ✅ | ✅ |
| TUI interface | ✅ | ✅ |
| JSON import/export | ✅ | ✅ |
| Hard link detection | ✅ | ✅ |
| Memory safety | GC | Compile-time |
| Binary size | ~8MB | ~3MB (estimated) |
| Runtime | Go runtime | Native |

### Project Structure

```
rdu/
├── Cargo.toml              # Workspace configuration
├── crates/
│   ├── rdu-cli/            # CLI entry point
│   ├── rdu-lib/            # Core library
│   │   ├── analyzer/       # Parallel/sequential scanners
│   │   ├── fs/             # File system types
│   │   ├── platform/       # Platform-specific code
│   │   ├── ignore/         # Ignore patterns
│   │   ├── timefilter/     # Time-based filtering
│   │   └── export/         # JSON import/export
│   └── rdu-tui/            # Terminal UI
└── tests/                  # Integration tests
```

### Technical Highlights

- **Rust 2021 Edition** with modern language features
- **Zero-cost abstractions** for performance
- **Thread-safe data structures** using `Arc`, `RwLock`, and atomics
- **Platform-specific optimizations** for Linux and Windows
- **Comprehensive error handling** with `thiserror` and `anyhow`
- **Builder pattern** for configuration
- **Extensive test coverage** with unit and integration tests

[0.1.0]: https://github.com/user/rdu/releases/tag/v0.1.0