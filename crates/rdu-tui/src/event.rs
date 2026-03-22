//! Event handling for the TUI.

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind, MouseEvent};
use std::time::Duration;

/// TUI event types.
#[derive(Debug)]
pub enum Event {
    /// Key press event
    Key(KeyEvent),
    /// Mouse event
    Mouse(MouseEvent),
    /// Terminal resize
    Resize(u16, u16),
    /// Tick event (for periodic updates)
    Tick,
}

/// Event handler for the TUI.
pub struct EventHandler {
    /// Tick rate for periodic updates
    tick_rate: Duration,
}

impl EventHandler {
    /// Create a new event handler with the given tick rate.
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    /// Wait for the next event.
    pub fn next(&self) -> Result<Event, std::io::Error> {
        if event::poll(self.tick_rate)? {
            match event::read()? {
                CrosstermEvent::Key(key) => {
                    // Only handle Press events to avoid double-triggering
                    if key.kind == KeyEventKind::Press {
                        Ok(Event::Key(key))
                    } else {
                        Ok(Event::Tick)
                    }
                }
                CrosstermEvent::Mouse(mouse) => Ok(Event::Mouse(mouse)),
                CrosstermEvent::Resize(cols, rows) => Ok(Event::Resize(cols, rows)),
                _ => Ok(Event::Tick),
            }
        } else {
            Ok(Event::Tick)
        }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new(Duration::from_millis(250))
    }
}
