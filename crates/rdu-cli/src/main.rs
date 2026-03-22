//! rdu - Fast disk usage analyzer written in Rust.
//!
//! A rewrite of gdu (Go Disk Usage) with similar functionality.

use anyhow::{Context, Result};
use clap::Parser;
use rdu_lib::{
    export_to_file, get_devices, import_from_file, parse_date, parse_duration,
    AnalyzerConfig, IgnoreMatcher, ParallelAnalyzer, SequentialAnalyzer, SortBy, SortOrder,
    TimeFilter,
};
use std::path::PathBuf;
use std::time::Duration;

/// Fast disk usage analyzer written in Rust.
#[derive(Parser, Debug)]
#[command(name = "rdu")]
#[command(author, version, about, long_about = None)]
#[command(next_line_help = true)]
struct Args {
    /// Directory to analyze (default: current directory)
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,

    // Display options
    /// Show apparent size instead of disk usage
    #[arg(short, long)]
    show_apparent_size: bool,

    /// Show relative size bars
    #[arg(short = 'B', long)]
    show_relative_size: bool,

    /// Show item count column
    #[arg(long)]
    show_item_count: bool,

    /// Show modification time column
    #[arg(long)]
    show_mtime: bool,

    /// Use SI prefixes (kB, MB) instead of binary (KiB, MiB)
    #[arg(long)]
    si: bool,

    // Analysis options
    /// Use sequential scanning (for HDDs)
    #[arg(long)]
    sequential: bool,

    /// Maximum number of CPU cores to use (0 = all available)
    #[arg(short = 'm', long, value_name = "N", default_value = "0")]
    max_cores: usize,

    /// Follow symlinks for files
    #[arg(short = 'L', long)]
    follow_symlinks: bool,

    /// Don't cross filesystem boundaries
    #[arg(short = 'x', long)]
    no_cross: bool,

    // Ignore options
    /// Directories to ignore (comma-separated)
    #[arg(short = 'i', long, value_name = "PATHS", value_delimiter = ',')]
    ignore_dirs: Vec<String>,

    /// Regex patterns for paths to ignore
    #[arg(short = 'I', long, value_name = "PATTERNS", value_delimiter = ',')]
    ignore_dirs_pattern: Vec<String>,

    /// Ignore hidden directories
    #[arg(short = 'H', long)]
    no_hidden: bool,

    // Output options
    /// Non-interactive mode (print results to stdout)
    #[arg(short, long)]
    non_interactive: bool,

    /// Show only total (implies non-interactive)
    #[arg(short, long)]
    summarize: bool,

    /// Export analysis to JSON file
    #[arg(short, long, value_name = "FILE")]
    output_file: Option<PathBuf>,

    /// Import analysis from JSON file
    #[arg(short, long, value_name = "FILE")]
    input_file: Option<PathBuf>,

    /// Show top N largest files/directories
    #[arg(short, long, value_name = "N")]
    top: Option<usize>,

    /// Sort by: size, name, count, mtime
    #[arg(long, value_name = "BY")]
    sort_by: Option<String>,

    /// Show list of mounted devices
    #[arg(short, long)]
    show_disks: bool,

    // Time filter options
    /// Show only items modified after this date
    #[arg(long, value_name = "DATE")]
    since: Option<String>,

    /// Show only items modified before this date
    #[arg(long, value_name = "DATE")]
    until: Option<String>,

    /// Show only items older than this (e.g., "30d", "1y")
    #[arg(long, value_name = "AGE")]
    max_age: Option<String>,

    /// Show only items newer than this (e.g., "7d", "1m")
    #[arg(long, value_name = "AGE")]
    min_age: Option<String>,

    // Other options
    /// Configuration file path
    #[arg(short = 'c', long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Log errors to stderr instead of ignoring them
    #[arg(long)]
    log_errors: bool,

    /// Don't show progress in non-interactive mode
    #[arg(long)]
    no_progress: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Show devices and exit
    if args.show_disks {
        show_devices()?;
        return Ok(());
    }

    // Import from file if specified
    if let Some(input_file) = &args.input_file {
        let root = import_from_file(input_file).context("Failed to import from file")?;

        if args.non_interactive || args.summarize {
            print_items(&root, &args)?;
        } else {
            run_tui_with_root(root, args.show_apparent_size)?;
        }
        return Ok(());
    }

    // Get path to analyze
    let path = args.path.clone().unwrap_or_else(|| PathBuf::from("."));

    // Build ignore matcher
    let ignore = build_ignore_matcher(&args)?;

    // Build time filter
    let time_filter = build_time_filter(&args)?;

    // Build analyzer config
    let config = AnalyzerConfig {
        follow_symlinks: args.follow_symlinks,
        no_cross: args.no_cross,
        ignore,
        max_threads: args.max_cores,
        time_filter,
    };

    // Run analysis
    let root = if args.sequential {
        let mut analyzer = SequentialAnalyzer::new(config);
        analyzer
            .analyze(&path)
            .context("Failed to analyze directory")?
    } else {
        let mut analyzer = ParallelAnalyzer::new(config);
        analyzer
            .analyze(&path)
            .context("Failed to analyze directory")?
    };

    // Export to file if specified
    if let Some(output_file) = &args.output_file {
        let root_item: rdu_lib::ItemRef = root.clone() as rdu_lib::ItemRef;
        export_to_file(&root_item, output_file).context("Failed to export to file")?;
        println!("Analysis exported to {}", output_file.display());
    }

    // Output results
    if args.non_interactive || args.summarize || args.top.is_some() {
        let root_item: rdu_lib::ItemRef = root.clone() as rdu_lib::ItemRef;
        print_items(&root_item, &args)?;
    } else {
        // Run TUI
        run_tui(&path, args.show_apparent_size)?;
    }

    Ok(())
}

/// Build the ignore matcher from CLI arguments.
fn build_ignore_matcher(args: &Args) -> Result<IgnoreMatcher> {
    let mut ignore = IgnoreMatcher::new()
        .with_paths(args.ignore_dirs.clone())
        .with_ignore_hidden(args.no_hidden);

    if !args.ignore_dirs_pattern.is_empty() {
        ignore = ignore
            .with_patterns(args.ignore_dirs_pattern.clone())
            .map_err(|e| anyhow::anyhow!("Invalid ignore pattern: {}", e))?;
    }

    Ok(ignore)
}

/// Build the time filter from CLI arguments.
fn build_time_filter(args: &Args) -> Result<TimeFilter> {
    let mut filter = TimeFilter::new();

    // Parse since date
    if let Some(since) = &args.since {
        let time = parse_date(since)
            .map_err(|e| anyhow::anyhow!("Invalid since date: {}", e))?;
        filter = filter.with_since(time);
    }

    // Parse until date
    if let Some(until) = &args.until {
        let time = parse_date(until)
            .map_err(|e| anyhow::anyhow!("Invalid until date: {}", e))?;
        filter = filter.with_until(time);
    }

    // Parse max_age (files must be older than this)
    if let Some(max_age) = &args.max_age {
        let duration = parse_duration(max_age)
            .map_err(|e| anyhow::anyhow!("Invalid max-age: {}", e))?;
        filter = filter.with_max_age(duration);
    }

    // Parse min_age (files must be newer than this)
    if let Some(min_age) = &args.min_age {
        let duration = parse_duration(min_age)
            .map_err(|e| anyhow::anyhow!("Invalid min-age: {}", e))?;
        filter = filter.with_min_age(duration);
    }

    Ok(filter)
}

/// Print items in non-interactive mode.
fn print_items(root: &rdu_lib::ItemRef, args: &Args) -> Result<()> {
    if args.summarize {
        print_summary(root, args);
        return Ok(());
    }

    let files = root
        .clone()
        .as_any()
        .downcast_ref::<rdu_lib::Dir>()
        .map(|d| {
            let mut files: Vec<_> = d.files_read().iter().cloned().collect();
            let sort_by = match args.sort_by.as_deref() {
                Some("size") => SortBy::Size,
                Some("name") => SortBy::Name,
                Some("count") => SortBy::ItemCount,
                Some("mtime") => SortBy::Mtime,
                _ => SortBy::Usage,
            };
            rdu_lib::sort_items(&mut files, sort_by, SortOrder::Desc);
            files
        })
        .unwrap_or_default();

    let display_count = args.top.unwrap_or(files.len());

    for item in files.iter().take(display_count) {
        print_item(item, args);
    }

    Ok(())
}

/// Print a single item.
fn print_item(item: &rdu_lib::ItemRef, args: &Args) {
    let size = if args.show_apparent_size {
        item.size()
    } else {
        item.usage()
    };

    let size_str = format_size(size, args.si);
    let name = item.display_name();
    let flag = item.flag().as_char();

    if args.show_item_count && item.is_dir() {
        println!("{:>10} {:>8} {}", size_str, item.item_count(), name);
    } else if flag != ' ' {
        println!("{:>10} {} {}", size_str, flag, name);
    } else {
        println!("{:>10} {}", size_str, name);
    }
}

/// Print a summary of the analysis.
fn print_summary(root: &rdu_lib::ItemRef, args: &Args) {
    let size = if args.show_apparent_size {
        root.size()
    } else {
        root.usage()
    };

    let size_str = format_size(size, args.si);
    let count = root.item_count();

    println!("{} items, total: {}", count, size_str);
}

/// Format a size in bytes to a human-readable string.
fn format_size(size: u64, use_si: bool) -> String {
    if size == 0 {
        return "0 B".to_string();
    }

    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    const UNITS_SI: &[&str] = &["B", "kB", "MB", "GB", "TB", "PB"];

    let units = if use_si { UNITS_SI } else { UNITS };
    let base = if use_si { 1000.0 } else { 1024.0 };

    let mut value = size as f64;
    let mut unit_index = 0;

    while value >= base && unit_index < units.len() - 1 {
        value /= base;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size, units[0])
    } else {
        format!("{:.1} {}", value, units[unit_index])
    }
}

/// Show mounted devices.
fn show_devices() -> Result<()> {
    let devices = get_devices().context("Failed to get device list")?;

    if devices.is_empty() {
        println!("No devices found or platform not supported.");
        return Ok(());
    }

    println!(
        "{:<20} {:<30} {:>12} {:>12} {:>12}",
        "Device", "Mount Point", "Total", "Used", "Free"
    );
    println!("{}", "-".repeat(88));

    for dev in devices {
        println!(
            "{:<20} {:<30} {:>12} {:>12} {:>12}",
            dev.name,
            dev.mount_point,
            format_size(dev.total_size, false),
            format_size(dev.used_size, false),
            format_size(dev.free_size, false),
        );
    }

    Ok(())
}

/// Run the TUI.
fn run_tui(path: &std::path::Path, show_apparent_size: bool) -> Result<()> {
    rdu_tui::run(path, show_apparent_size).context("Failed to run TUI")
}

/// Run the TUI with a pre-analyzed root.
fn run_tui_with_root(_root: rdu_lib::ItemRef, _show_apparent_size: bool) -> Result<()> {
    // TODO: Implement TUI with pre-loaded data
    println!("TUI with pre-loaded data not yet implemented");
    Ok(())
}
