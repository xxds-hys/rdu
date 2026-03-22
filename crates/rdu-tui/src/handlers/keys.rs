//! Key event handling.

use crate::app::App;
use super::actions::KeyAction;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle a key event and return the resulting action.
pub fn handle_key(key: KeyEvent, app: &mut App) -> KeyAction {
    // Handle modals first
    if app.is_showing_help() {
        return handle_help_modal_key(key, app);
    }

    if app.is_confirming_delete() {
        return handle_confirm_modal_key(key, app);
    }

    // Normal key handling
    match (key.modifiers, key.code) {
        // Quit
        (KeyModifiers::CONTROL, KeyCode::Char('c')) | (_, KeyCode::Char('q')) => KeyAction::Quit,

        // Navigation
        (_, KeyCode::Up) | (_, KeyCode::Char('k')) => {
            app.move_up();
            KeyAction::Continue
        }
        (_, KeyCode::Down) | (_, KeyCode::Char('j')) => {
            app.move_down();
            KeyAction::Continue
        }
        (_, KeyCode::Left) | (_, KeyCode::Char('h')) => {
            app.go_to_parent();
            KeyAction::Continue
        }
        (_, KeyCode::Right) | (_, KeyCode::Char('l')) | (_, KeyCode::Enter) => {
            app.enter_selected();
            KeyAction::Continue
        }
        (_, KeyCode::Home) | (_, KeyCode::Char('g')) => {
            app.move_to_top();
            KeyAction::Continue
        }
        (_, KeyCode::End) | (_, KeyCode::Char('G')) => {
            app.move_to_bottom();
            KeyAction::Continue
        }
        (_, KeyCode::PageUp) => {
            app.page_up(20);
            KeyAction::Continue
        }
        (_, KeyCode::PageDown) => {
            app.page_down(20);
            KeyAction::Continue
        }

        // Actions
        (_, KeyCode::Char('d')) => {
            app.request_delete();
            KeyAction::Continue
        }
        (_, KeyCode::Char(' ')) => {
            app.toggle_mark();
            app.move_down();
            KeyAction::Continue
        }
        (_, KeyCode::Char('r')) => KeyAction::Rescan,

        // Display toggles
        (_, KeyCode::Char('a')) => {
            app.toggle_apparent_size();
            KeyAction::Continue
        }
        (_, KeyCode::Char('c')) => {
            app.toggle_item_count();
            KeyAction::Continue
        }
        (_, KeyCode::Char('m')) => {
            app.toggle_mtime();
            KeyAction::Continue
        }
        (_, KeyCode::Char('s')) => {
            app.toggle_sort_by();
            KeyAction::Continue
        }
        (_, KeyCode::Char('S')) => {
            app.toggle_sort_order();
            KeyAction::Continue
        }

        // Filter
        (_, KeyCode::Char('/')) => {
            // TODO: Implement filter input
            KeyAction::Continue
        }
        (_, KeyCode::Esc) => {
            app.set_filter(None);
            KeyAction::Continue
        }

        // Help
        (_, KeyCode::Char('?')) => {
            app.toggle_help();
            KeyAction::Continue
        }

        _ => KeyAction::Continue,
    }
}

/// Handle key events in the help modal.
fn handle_help_modal_key(key: KeyEvent, app: &mut App) -> KeyAction {
    match key.code {
        KeyCode::Char('?') | KeyCode::Esc | KeyCode::Enter => {
            app.toggle_help();
            KeyAction::Continue
        }
        _ => KeyAction::Continue,
    }
}

/// Handle key events in the confirm modal.
fn handle_confirm_modal_key(key: KeyEvent, app: &mut App) -> KeyAction {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Err(e) = app.confirm_delete() {
                eprintln!("Error: {}", e);
            }
            KeyAction::Continue
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.cancel_delete();
            KeyAction::Continue
        }
        _ => KeyAction::Continue,
    }
}
