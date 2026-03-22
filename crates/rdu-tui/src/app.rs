//! Main TUI application.

use crate::event::{Event, EventHandler};
use crate::handlers::{handle_key, KeyAction};
use crate::ui;
use anyhow::{Context, Result};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use parking_lot::RwLock;
use ratatui::{backend::CrosstermBackend, Terminal};
use rdu_lib::{
    AnalyzerConfig, DirRef, Item, ItemFlag, ItemRef, ParallelAnalyzer, SortBy, SortOrder,
};
use std::collections::HashSet;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

/// TUI application state.
pub struct App {
    /// Current directory being displayed
    current_dir: Option<ItemRef>,
    /// Root directory (for going back to top)
    root_dir: Option<DirRef>,
    /// Currently selected row index
    selected: usize,
    /// Rows that have been marked for deletion
    marked: HashSet<usize>,
    /// Current sort criteria
    sort_by: SortBy,
    /// Current sort order
    sort_order: SortOrder,
    /// Show apparent size instead of disk usage
    show_apparent_size: bool,
    /// Show item count column
    show_item_count: bool,
    /// Show modification time column
    show_mtime: bool,
    /// Filter pattern (if any)
    filter: Option<String>,
    /// Help modal is visible
    show_help: bool,
    /// Confirmation modal is visible
    confirm_delete: bool,
    /// Items pending deletion
    pending_delete: Vec<ItemRef>,
    /// Error message (if any)
    error: Option<String>,
    /// Path being analyzed
    path: std::path::PathBuf,
    /// Scanning state
    is_scanning: bool,
    /// Scroll offset for the file list
    scroll_offset: usize,
}

impl App {
    /// Create a new TUI application.
    pub fn new(path: &Path, show_apparent_size: bool) -> Result<Self> {
        let mut app = Self {
            current_dir: None,
            root_dir: None,
            selected: 0,
            marked: HashSet::new(),
            sort_by: SortBy::Usage,
            sort_order: SortOrder::Desc,
            show_apparent_size,
            show_item_count: false,
            show_mtime: false,
            filter: None,
            show_help: false,
            confirm_delete: false,
            pending_delete: Vec::new(),
            error: None,
            path: path.to_path_buf(),
            is_scanning: true,
            scroll_offset: 0,
        };

        // Start scanning
        app.scan()?;

        Ok(app)
    }

    /// Scan the configured path.
    fn scan(&mut self) -> Result<()> {
        self.is_scanning = true;

        let config = AnalyzerConfig::default();
        let mut analyzer = ParallelAnalyzer::new(config);

        let result = analyzer
            .analyze(&self.path)
            .context("Failed to analyze path")?;

        self.root_dir = Some(result.clone());
        self.current_dir = Some(result);
        self.selected = 0;
        self.marked.clear();
        self.is_scanning = false;

        Ok(())
    }

    /// Run the TUI application.
    pub fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode().context("Failed to enable raw mode")?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
            .context("Failed to setup terminal")?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).context("Failed to create terminal backend")?;

        // Create event handler
        let events = EventHandler::new(Duration::from_millis(250));

        // Main loop
        let res = self.main_loop(&mut terminal, events);

        // Restore terminal
        disable_raw_mode().context("Failed to disable raw mode")?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .context("Failed to restore terminal")?;
        terminal.show_cursor().context("Failed to show cursor")?;

        res
    }

    /// Main event loop.
    fn main_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        events: EventHandler,
    ) -> Result<()> {
        loop {
            // Draw UI
            terminal
                .draw(|f| ui::draw(f, self))
                .context("Failed to draw UI")?;

            // Handle events
            match events.next().context("Failed to get next event")? {
                Event::Key(key) => {
                    let action = handle_key(key, self);
                    match action {
                        KeyAction::Quit => break,
                        KeyAction::Continue => {}
                        KeyAction::Rescan => {
                            if let Err(e) = self.scan() {
                                self.error = Some(e.to_string());
                            }
                        }
                    }
                }
                Event::Resize(_, _) => {
                    // Terminal will be redrawn on next iteration
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Get the current directory.
    pub fn current_dir(&self) -> Option<&ItemRef> {
        self.current_dir.as_ref()
    }

    /// Get the list of files in the current directory, sorted and filtered.
    pub fn get_files(&self) -> Vec<ItemRef> {
        if let Some(dir) = &self.current_dir {
            if let Some(dir_ref) = dir.as_any().downcast_ref::<rdu_lib::Dir>() {
                let mut files: Vec<ItemRef> = dir_ref
                    .files_read()
                    .iter()
                    .filter(|f| {
                        if let Some(filter) = &self.filter {
                            f.name().to_lowercase().contains(&filter.to_lowercase())
                        } else {
                            true
                        }
                    })
                    .cloned()
                    .collect();

                rdu_lib::sort_items(&mut files, self.sort_by, self.sort_order);
                files
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    /// Move selection up.
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down.
    pub fn move_down(&mut self) {
        let files = self.get_files();
        if self.selected + 1 < files.len() {
            self.selected += 1;
        }
    }

    /// Move to the top of the list.
    pub fn move_to_top(&mut self) {
        self.selected = 0;
        self.scroll_offset = 0;
    }

    /// Move to the bottom of the list.
    pub fn move_to_bottom(&mut self) {
        let files = self.get_files();
        if !files.is_empty() {
            self.selected = files.len() - 1;
        }
    }

    /// Page up.
    pub fn page_up(&mut self, page_size: usize) {
        self.selected = self.selected.saturating_sub(page_size);
    }

    /// Page down.
    pub fn page_down(&mut self, page_size: usize) {
        let files = self.get_files();
        self.selected = (self.selected + page_size).min(files.len().saturating_sub(1));
    }

    /// Enter the selected directory.
    pub fn enter_selected(&mut self) {
        let files = self.get_files();
        if let Some(item) = files.get(self.selected).cloned() {
            if item.is_dir() {
                self.current_dir = Some(item);
                self.selected = 0;
                self.scroll_offset = 0;
                self.marked.clear();
            }
        }
    }

    /// Go to parent directory.
    pub fn go_to_parent(&mut self) {
        if let Some(current) = &self.current_dir {
            // Save the current directory name before changing
            let current_name = current.name().to_string();

            if let Some(dir_ref) = current.as_any().downcast_ref::<rdu_lib::Dir>() {
                if let Some(parent_weak) = dir_ref.parent() {
                    if let Some(parent) = parent_weak.upgrade() {
                        // Convert DirRef to ItemRef and set as current
                        self.current_dir = Some(parent.clone() as ItemRef);
                        self.scroll_offset = 0;
                        self.marked.clear();

                        // Find the previous directory in the sorted file list
                        let files = self.get_files();
                        for (i, f) in files.iter().enumerate() {
                            if f.name() == current_name {
                                self.selected = i;
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Toggle mark on the selected item.
    pub fn toggle_mark(&mut self) {
        if self.marked.contains(&self.selected) {
            self.marked.remove(&self.selected);
        } else {
            self.marked.insert(self.selected);
        }
    }

    /// Toggle sort by.
    pub fn toggle_sort_by(&mut self) {
        self.sort_by = self.sort_by.next();
    }

    /// Toggle sort order.
    pub fn toggle_sort_order(&mut self) {
        self.sort_order = self.sort_order.toggle();
    }

    /// Toggle apparent size display.
    pub fn toggle_apparent_size(&mut self) {
        self.show_apparent_size = !self.show_apparent_size;
    }

    /// Toggle item count display.
    pub fn toggle_item_count(&mut self) {
        self.show_item_count = !self.show_item_count;
    }

    /// Toggle modification time display.
    pub fn toggle_mtime(&mut self) {
        self.show_mtime = !self.show_mtime;
    }

    /// Show/hide help.
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// Set filter pattern.
    pub fn set_filter(&mut self, filter: Option<String>) {
        self.filter = filter;
        self.selected = 0;
        self.scroll_offset = 0;
    }

    /// Get the selected item.
    pub fn get_selected_item(&self) -> Option<ItemRef> {
        self.get_files().get(self.selected).cloned()
    }

    /// Get marked items.
    pub fn get_marked_items(&self) -> Vec<ItemRef> {
        let files = self.get_files();
        self.marked
            .iter()
            .filter_map(|&i| files.get(i).cloned())
            .collect()
    }

    /// Request deletion confirmation.
    pub fn request_delete(&mut self) {
        let items = if self.marked.is_empty() {
            vec![self.selected]
        } else {
            self.marked.iter().copied().collect()
        };

        let files = self.get_files();
        self.pending_delete = items
            .iter()
            .filter_map(|&i| files.get(i).cloned())
            .collect();

        if !self.pending_delete.is_empty() {
            self.confirm_delete = true;
        }
    }

    /// Confirm deletion.
    pub fn confirm_delete(&mut self) -> Result<()> {
        for item in &self.pending_delete {
            let path = item.path();
            if item.is_dir() {
                std::fs::remove_dir_all(&path)
                    .with_context(|| format!("Failed to delete directory: {}", path))?;
            } else {
                std::fs::remove_file(&path)
                    .with_context(|| format!("Failed to delete file: {}", path))?;
            }
        }

        // Remove from current directory
        if let Some(dir) = &self.current_dir {
            if let Some(dir_ref) = dir.as_any().downcast_ref::<rdu_lib::Dir>() {
                for item in &self.pending_delete {
                    dir_ref.remove_file(item);
                }
                dir_ref.update_stats();
            }
        }

        self.pending_delete.clear();
        self.confirm_delete = false;
        self.marked.clear();
        self.selected = 0;

        Ok(())
    }

    /// Cancel deletion.
    pub fn cancel_delete(&mut self) {
        self.pending_delete.clear();
        self.confirm_delete = false;
    }

    /// Check if showing help.
    pub fn is_showing_help(&self) -> bool {
        self.show_help
    }

    /// Check if confirming delete.
    pub fn is_confirming_delete(&self) -> bool {
        self.confirm_delete
    }

    /// Get pending delete items.
    pub fn get_pending_delete(&self) -> &[ItemRef] {
        &self.pending_delete
    }

    /// Get current sort by.
    pub fn sort_by(&self) -> SortBy {
        self.sort_by
    }

    /// Get current sort order.
    pub fn sort_order(&self) -> SortOrder {
        self.sort_order
    }

    /// Check if showing apparent size.
    pub fn is_showing_apparent_size(&self) -> bool {
        self.show_apparent_size
    }

    /// Check if showing item count.
    pub fn is_showing_item_count(&self) -> bool {
        self.show_item_count
    }

    /// Check if showing mtime.
    pub fn is_showing_mtime(&self) -> bool {
        self.show_mtime
    }

    /// Get current selection index.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Check if an index is marked.
    pub fn is_marked(&self, index: usize) -> bool {
        self.marked.contains(&index)
    }

    /// Get the filter pattern.
    pub fn filter(&self) -> Option<&str> {
        self.filter.as_deref()
    }

    /// Check if scanning.
    pub fn is_scanning(&self) -> bool {
        self.is_scanning
    }

    /// Get the path being analyzed.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get scroll offset.
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Set scroll offset.
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
    }
}
