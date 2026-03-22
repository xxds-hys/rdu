//! TUI module for rdu.

mod app;
mod event;
mod handlers;
mod ui;

pub use app::App;
pub use event::{Event, EventHandler};

use anyhow::Result;

/// Run the TUI application.
pub fn run(path: &std::path::Path, show_apparent_size: bool) -> Result<()> {
    let mut app = App::new(path, show_apparent_size)?;
    app.run()
}
