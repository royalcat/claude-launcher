//! Library facade for `claude-launcher`.
//!
//! Mirrors the module tree declared in `main.rs` so the binary and the crate are both
//! compilable and integration tests in `tests/` can exercise the public API directly
//! against a `ratatui` TestBackend (no real terminal needed).

pub mod actions;
pub mod cli;
pub mod config;
pub mod error;
pub mod providers;
pub mod settings;
pub mod statusline;
pub mod tui;
