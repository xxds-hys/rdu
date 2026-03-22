# rdu - Rust Disk Usage

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

A fast disk usage analyzer written in Rust, inspired by [gdu](https://github.com/dundee/gdu).

English | [中文文档](#中文文档)

---

## Features

- **Fast parallel scanning** - Utilizes all CPU cores for maximum performance
- **Interactive TUI** - Navigate and explore disk usage with an intuitive terminal interface
- **Cross-platform** - Works on Linux, Windows, and macOS
- **JSON import/export** - Save and load analysis results
- **Flexible filtering** - Ignore patterns, hidden files, filesystem boundaries
- **Time filtering** - Filter files by modification time
- **Hard link detection** - Avoids double-counting hard links
- **Memory safe** - Rust's ownership system guarantees memory safety

## Performance

Benchmarks show rdu is **24-28% faster** than gdu on large directories:

| Directory | rdu | gdu | Improvement |
|-----------|-----|-----|-------------|
| C:/Windows (34GB) | 0.90s | 1.22s | **26% faster** |
| C:/Users (76GB) | 0.82s | 1.14s | **28% faster** |
| User directories | 0.07s | 0.18s | **61% faster** |

See [BENCHMARK.md](BENCHMARK.md) for detailed results.

## Installation

### From Source

```bash
git clone https://github.com/xxds-hys/rdu.git
cd rdu
cargo build --release
```

The binary will be at `target/release/rdu`

### Pre-built Binaries

Download from [Releases](https://github.com/xxds-hys/rdu/releases) page.

## Usage

### Interactive Mode (default)

```bash
rdu /path/to/analyze
```

### Non-Interactive Mode

```bash
rdu -n /path/to/analyze
rdu -n --top 10 /path/to/analyze
```

### Export/Import JSON

```bash
rdu -o report.json /path/to/analyze
rdu -f report.json
```

### Time Filtering

```bash
rdu --min-age 7d /path          # Modified in last 7 days
rdu --max-age 30d /path         # Older than 30 days
rdu --since 2024-01-01 /path    # Since specific date
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `↑/k` `↓/j` | Move up/down |
| `←/h` | Go to parent directory |
| `→/l/Enter` | Enter directory |
| `d` | Delete selected items |
| `Space` | Toggle mark |
| `a` | Toggle apparent size |
| `c` | Toggle item count |
| `m` | Toggle modification time |
| `s` | Cycle sort by |
| `r` | Rescan directory |
| `?` | Show help |
| `q` | Quit |

## Comparison with gdu

| Feature | gdu (Go) | rdu (Rust) |
|---------|----------|------------|
| Parallel scanning | ✅ | ✅ |
| TUI interface | ✅ | ✅ |
| JSON import/export | ✅ | ✅ |
| Hard link detection | ✅ | ✅ |
| Time filtering | ❌ | ✅ |
| Archive browsing | ✅ | ❌ |
| Binary size | ~8MB | ~3MB |
| Memory safety | GC | Compile-time |

## Project Structure

```
rdu/
├── crates/
│   ├── rdu-cli/     # CLI entry point
│   ├── rdu-lib/     # Core library (analyzer, fs, platform)
│   └── rdu-tui/     # Terminal UI
└── tests/           # Integration tests
```

## Development

```bash
cargo build          # Build
cargo test           # Run tests
cargo run -- /path   # Run
```

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

Inspired by [gdu](https://github.com/dundee/gdu) by Daniel Milde.

---

# 中文文档

一个用 Rust 编写的快速磁盘使用分析工具，灵感来自 [gdu](https://github.com/dundee/gdu)。

## 特性

- **快速并行扫描** - 利用所有 CPU 核心实现最大性能
- **交互式 TUI** - 直观的终端界面浏览磁盘使用情况
- **跨平台** - 支持 Linux、Windows 和 macOS
- **JSON 导入/导出** - 保存和加载分析结果
- **灵活过滤** - 忽略模式、隐藏文件、文件系统边界
- **时间过滤** - 按修改时间筛选文件
- **硬链接检测** - 避免重复计算

## 性能

基准测试显示 rdu 比 gdu 快 **24-28%**：

| 目录 | rdu | gdu | 提升 |
|------|-----|-----|------|
| C:/Windows (34GB) | 0.90s | 1.22s | **快 26%** |
| C:/Users (76GB) | 0.82s | 1.14s | **快 28%** |
| 用户目录 | 0.07s | 0.18s | **快 61%** |

## 安装

```bash
git clone https://github.com/xxds-hys/rdu.git
cd rdu
cargo build --release
```

## 使用

```bash
# 交互模式
rdu /path/to/analyze

# 非交互模式
rdu -n /path/to/analyze

# 显示最大的 10 个文件
rdu -n --top 10 /path

# 导出 JSON 报告
rdu -o report.json /path
```

## 快捷键

| 按键 | 功能 |
|------|------|
| `↑/k` `↓/j` | 上下移动 |
| `←/h` | 返回上级目录 |
| `→/l/Enter` | 进入目录 |
| `d` | 删除选中项 |
| `Space` | 标记/取消标记 |
| `a` | 切换实际大小显示 |
| `c` | 切换项目数量显示 |
| `m` | 切换修改时间显示 |
| `q` | 退出 |

## 许可证

MIT 许可证 - 详见 [LICENSE](LICENSE)。