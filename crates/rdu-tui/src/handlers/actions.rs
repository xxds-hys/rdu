//! Key action types.

/// Result of handling a key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    /// Continue running the application
    Continue,
    /// Quit the application
    Quit,
    /// Rescan the current directory
    Rescan,
}
