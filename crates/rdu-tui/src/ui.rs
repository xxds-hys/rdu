//! UI rendering for the TUI.

use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use rdu_lib::{Item, ItemFlag, SortBy, SortOrder};
use std::time::SystemTime;

/// Size units for human-readable formatting.
const SIZE_UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];
const SIZE_UNITS_SI: &[&str] = &["B", "kB", "MB", "GB", "TB", "PB", "EB"];

/// Format a size in bytes to a human-readable string.
pub fn format_size(size: u64, use_si: bool) -> String {
    if size == 0 {
        return "0 B".to_string();
    }

    let units = if use_si { SIZE_UNITS_SI } else { SIZE_UNITS };
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

/// Format modification time to a readable string.
pub fn format_mtime(mtime: SystemTime) -> String {
    let now = SystemTime::now();

    // Calculate time elapsed since modification
    match now.duration_since(mtime) {
        Ok(elapsed) => {
            let secs = elapsed.as_secs();
            let days = secs / 86400;
            let years = days / 365;

            if years > 0 {
                format!("{}y ago", years)
            } else if days > 0 {
                format!("{}d ago", days)
            } else {
                let hours = (secs % 86400) / 3600;
                if hours > 0 {
                    format!("{}h ago", hours)
                } else {
                    let mins = (secs % 3600) / 60;
                    if mins > 0 {
                        format!("{}m ago", mins)
                    } else {
                        let secs = secs % 60;
                        format!("{}s ago", secs)
                    }
                }
            }
        }
        Err(_) => {
            // File was modified in the future (clock issue), show actual date
            "future".to_string()
        }
    }
}

/// Draw the main UI.
pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // File list
            Constraint::Length(1), // Footer
        ])
        .split(f.area());

    // Draw header
    draw_header(f, app, chunks[0]);

    // Draw file list
    draw_file_list(f, app, chunks[1]);

    // Draw footer
    draw_footer(f, app, chunks[2]);

    // Draw help modal if visible
    if app.is_showing_help() {
        draw_help_modal(f);
    }

    // Draw confirm modal if visible
    if app.is_confirming_delete() {
        draw_confirm_modal(f, app);
    }
}

/// Draw the header.
fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let path = app.path().display().to_string();
    let current_path = if let Some(dir) = app.current_dir() {
        format!("{}: {}", path, dir.path())
    } else if app.is_scanning() {
        format!("{}: Scanning...", path)
    } else {
        path
    };

    let sort_info = format!(
        "Sort: {} ({})",
        app.sort_by().display_name(),
        app.sort_order().display_name()
    );

    let header = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            &current_path,
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::raw(sort_info),
            Span::raw("  "),
            if app.is_showing_apparent_size() {
                Span::styled("[A]", Style::default().fg(Color::Yellow))
            } else {
                Span::raw("[a]")
            },
            Span::raw(" "),
            if app.is_showing_item_count() {
                Span::styled("[C]", Style::default().fg(Color::Yellow))
            } else {
                Span::raw("[c]")
            },
            Span::raw(" "),
            if app.is_showing_mtime() {
                Span::styled("[M]", Style::default().fg(Color::Yellow))
            } else {
                Span::raw("[m]")
            },
        ]),
    ])
    .block(Block::default().borders(Borders::BOTTOM));

    f.render_widget(header, area);
}

/// Draw the file list.
fn draw_file_list(f: &mut Frame, app: &App, area: Rect) {
    let files = app.get_files();
    let visible_height = area.height.saturating_sub(2) as usize; // Account for borders/padding

    // Calculate scroll offset to keep selected item visible
    let selected = app.selected();
    let scroll_offset = app.scroll_offset();

    // Compute new scroll offset
    let new_scroll_offset = if selected < scroll_offset {
        selected
    } else if visible_height > 0 && selected >= scroll_offset + visible_height {
        selected.saturating_sub(visible_height.saturating_sub(1))
    } else {
        scroll_offset
    };

    // Get the slice of items to display
    let display_items: Vec<ListItem> = files
        .iter()
        .enumerate()
        .skip(new_scroll_offset)
        .take(visible_height.max(1))
        .map(|(i, item)| {
            let is_selected = i == selected;
            let is_marked = app.is_marked(i);

            // Size
            let size = if app.is_showing_apparent_size() {
                format_size(item.size(), false)
            } else {
                format_size(item.usage(), false)
            };

            // Flag
            let flag = item.flag().as_char();

            // Name with directory indicator
            let name = if item.is_dir() {
                format!("{}/", item.name())
            } else {
                item.name().to_string()
            };

            // Build the line
            let style = if is_selected {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else if is_marked {
                Style::default().bg(Color::DarkGray)
            } else if item.has_error() {
                Style::default().fg(Color::Red)
            } else if item.is_dir() {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };

            let mut spans = vec![
                Span::styled(
                    if is_marked { "* " } else { "  " },
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    format!("{:>10} ", size),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ];

            // Optional columns
            if app.is_showing_item_count() {
                let count = if item.is_dir() {
                    format!("{:>6} ", item.item_count())
                } else {
                    "       ".to_string()
                };
                spans.push(Span::raw(count));
            }

            if app.is_showing_mtime() {
                let mtime = format_mtime(item.mtime());
                spans.push(Span::styled(
                    format!("{:>10} ", mtime),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            // Flag
            if flag != ' ' {
                spans.push(Span::styled(
                    format!("{} ", flag),
                    Style::default().fg(Color::Magenta),
                ));
            }

            // Name
            spans.push(Span::styled(name, style));

            ListItem::new(Line::from(spans)).style(style)
        })
        .collect();

    let list = List::new(display_items).block(Block::default());

    f.render_widget(list, area);
}

/// Draw the footer.
fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let total_size = if let Some(dir) = app.current_dir() {
        if app.is_showing_apparent_size() {
            format_size(dir.size(), false)
        } else {
            format_size(dir.usage(), false)
        }
    } else {
        "0 B".to_string()
    };

    let item_count = if let Some(dir) = app.current_dir() {
        dir.item_count()
    } else {
        0
    };

    let footer = Paragraph::new(Line::from(vec![
        Span::raw("Total: "),
        Span::styled(total_size, Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("  Items: "),
        Span::styled(
            item_count.to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("?", Style::default().fg(Color::Yellow)),
        Span::raw(" help  "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" quit"),
    ]));

    f.render_widget(footer, area);
}

/// Draw the help modal.
fn draw_help_modal(f: &mut Frame) {
    let area = centered_rect(60, 70, f.area());
    f.render_widget(Clear, area);

    let help_text = vec![
        Line::from(Span::styled(
            "Help - Keyboard Shortcuts",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::from(vec![Span::styled(
            "  Navigation:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::raw("    ↑/k      Move up"),
        Line::raw("    ↓/j      Move down"),
        Line::raw("    ←/h      Go to parent directory"),
        Line::raw("    →/l/Enter  Enter directory"),
        Line::raw("    Home/g   Go to top"),
        Line::raw("    End/G    Go to bottom"),
        Line::raw("    PgUp     Page up"),
        Line::raw("    PgDn     Page down"),
        Line::raw(""),
        Line::from(vec![Span::styled(
            "  Actions:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::raw("    d        Delete selected/marked items"),
        Line::raw("    Space    Toggle mark on selected item"),
        Line::raw("    r        Rescan current directory"),
        Line::raw(""),
        Line::from(vec![Span::styled(
            "  Display:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::raw("    a        Toggle apparent size"),
        Line::raw("    c        Toggle item count"),
        Line::raw("    m        Toggle modification time"),
        Line::raw("    s        Cycle sort by (size/name/count/mtime)"),
        Line::raw("    /        Filter by name"),
        Line::raw("    Esc      Clear filter"),
        Line::raw(""),
        Line::raw("    q        Quit"),
        Line::raw("    ?        Toggle this help"),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Draw the delete confirmation modal.
fn draw_confirm_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 30, f.area());
    f.render_widget(Clear, area);

    let items = app.get_pending_delete();
    let count = items.len();
    let total_size: u64 = items.iter().map(|i| i.usage()).sum();

    let text = vec![
        Line::from(Span::styled(
            "Confirm Deletion",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::from(format!(
            "Delete {} item(s) totaling {}?",
            count,
            format_size(total_size, false)
        )),
        Line::raw(""),
        Line::from(vec![
            Span::styled("y", Style::default().fg(Color::Yellow)),
            Span::raw(": Yes  "),
            Span::styled("n/Esc", Style::default().fg(Color::Yellow)),
            Span::raw(": No"),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black)),
        )
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(paragraph, area);
}

/// Create a centered rectangle.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
