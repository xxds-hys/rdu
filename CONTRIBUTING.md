# Contributing to rdu

Thank you for your interest in contributing to rdu! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Code Style](#code-style)
- [Commit Messages](#commit-messages)
- [Pull Requests](#pull-requests)

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment. Please be considerate of others and follow standard open-source community guidelines.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/rdu.git`
3. Create a feature branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Run tests: `cargo test`
6. Commit your changes: `git commit -am 'Add some feature'`
7. Push to the branch: `git push origin feature/your-feature-name`
8. Submit a pull request

## Development Setup

### Prerequisites

- Rust 1.70 or later
- Cargo (comes with Rust)
- Git

### Building

```bash
# Build all crates
cargo build

# Build in release mode
cargo build --release

# Build specific crate
cargo build -p rdu-lib
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p rdu-lib

# Run a specific test
cargo test test_parallel_analyzer_basic

# Run tests with output
cargo test -- --nocapture
```

### Running the CLI

```bash
# Run in development mode
cargo run -- /path/to/analyze

# Run with options
cargo run -- --help
cargo run -- -n /path/to/analyze
```

## Project Structure

```
rdu/
├── crates/
│   ├── rdu-cli/           # Command-line interface
│   │   └── src/
│   │       └── main.rs    # Entry point, argument parsing
│   │
│   ├── rdu-lib/           # Core library
│   │   └── src/
│   │       ├── analyzer/  # Scanning logic (parallel/sequential)
│   │       ├── fs/        # File system types (File, Dir, Item)
│   │       ├── platform/  # Platform-specific code
│   │       ├── ignore/    # Pattern matching for exclusions
│   │       ├── timefilter/# Time-based filtering
│   │       ├── export/    # JSON import/export
│   │       └── device/    # Device/mount information
│   │
│   └── rdu-tui/           # Terminal user interface
│       └── src/
│           ├── app.rs     # Application state
│           ├── ui.rs      # Rendering
│           └── handlers/  # Event handling
│
└── tests/                 # Integration tests
    └── integration_test.rs
```

## Making Changes

### Adding a New Feature

1. **Create an issue** describing the feature
2. **Discuss the approach** in the issue
3. **Implement the feature** following the project structure
4. **Add tests** for the new functionality
5. **Update documentation** if needed

### Fixing a Bug

1. **Create an issue** describing the bug
2. **Write a test** that reproduces the bug
3. **Fix the bug**
4. **Verify the test passes**

## Testing

### Unit Tests

Unit tests are placed in the same file as the code they test, inside a `#[cfg(test)]` module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_function() {
        // test code
    }
}
```

### Integration Tests

Integration tests are placed in the `tests/` directory at the project root.

### Test Guidelines

- Write tests for all new functionality
- Ensure existing tests pass before submitting PR
- Use descriptive test names: `test_<function>_<scenario>`
- Test edge cases and error conditions

## Code Style

### Rust Conventions

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` to format code
- Use `cargo clippy` for linting

### Documentation

- Document all public APIs with doc comments (`///`)
- Include examples in doc comments
- Update README.md for user-facing changes

### Example

```rust
/// Calculate the total size of files in a directory.
///
/// # Arguments
///
/// * `path` - Path to the directory
///
/// # Returns
///
/// The total size in bytes, or an error if the directory cannot be read.
///
/// # Example
///
/// ```
/// let size = calculate_total_size("/home/user")?;
/// println!("Total size: {} bytes", size);
/// ```
pub fn calculate_total_size(path: &Path) -> Result<u64, std::io::Error> {
    // implementation
}
```

## Commit Messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or modifying tests
- `chore`: Maintenance tasks

### Examples

```
feat(analyzer): add time-based file filtering

Add support for filtering files by modification time using
--since, --until, --max-age, and --min-age options.

Closes #123
```

```
fix(fs): handle permission denied errors gracefully

When a directory cannot be read due to permission issues,
mark it with a flag instead of failing the entire scan.
```

## Pull Requests

### Checklist

- [ ] Code compiles without warnings
- [ ] All tests pass
- [ ] New code has tests
- [ ] Documentation is updated
- [ ] Commit messages follow conventions
- [ ] Branch is up to date with main

### Review Process

1. Submit your PR
2. Wait for CI to pass
3. Address review comments
4. Once approved, a maintainer will merge

Thank you for contributing!