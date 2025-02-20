//! Main entry point for cyberorganism. This module coordinates
//! the application's state and event loop, delegating UI rendering, task operations, and
//! command handling to their respective specialized modules.

mod commands;
mod taskstore;
mod ui;

use crossterm::event::{self, Event, KeyCode};
use std::io;
use taskstore::{load_tasks, Task};

/// Central state container for the cyberorganism application.
pub struct App {
    /// Collection of all tasks in the system
    pub tasks: Vec<Task>,
    /// Current user input being typed
    pub input: String,
    /// Position of cursor within input string (in bytes)
    pub cursor_position: usize,
    /// Counter for generating the next unique task ID
    pub next_id: u32,
}

impl App {
    fn new() -> Self {
        // Implementation note: Try to load existing tasks, or start with empty vec if none exist
        let tasks = load_tasks().unwrap_or_default();
        // Implementation note: Find the highest task id to continue from
        let next_id = tasks.iter().map(|task| task.id).max().unwrap_or(0) + 1;

        Self {
            tasks,
            input: String::new(),
            cursor_position: 0,
            next_id,
        }
    }
}

/// Application entry point and main event loop.
///
/// Sets up the terminal UI, initializes the application state,
/// and processes user input until exit.
fn main() -> io::Result<()> {
    // Set up terminal
    let mut terminal = ui::setup_terminal()?;

    // Create app state
    let mut app = App::new();

    // Main loop
    loop {
        // Draw the current state of the app
        terminal.draw(|frame| {
            ui::draw(frame, &app)
        })?;

        // Handle input
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc => break,
                KeyCode::Char('c') if key.modifiers == event::KeyModifiers::CONTROL => break,
                _ => commands::handle_input(&mut app, key.code),
            }
        }
    }

    // Restore terminal
    ui::restore_terminal(&mut terminal)?;
    Ok(())
}
