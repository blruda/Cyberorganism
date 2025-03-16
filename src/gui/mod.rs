//! GUI implementation using egui.
//! 
//! This module provides the GUI implementation for the cyberorganism task manager.
//! It replaces the previous TUI implementation while maintaining the same
//! minimalist interface design.

mod keyhandler;
mod rendering;
pub mod genius_feed;

pub use rendering::run_app;
