//! Main entry point for cyberorganism. This module coordinates
//! the application's state and event loop, delegating UI rendering, task operations, and
//! command handling to their respective specialized modules.

mod commands;
mod debug;
mod taskstore;
mod ui;

use crate::ui::{ActivityLog, TaskpadState};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::Terminal;
use std::io;
use std::time::Duration;
use taskstore::{load_tasks, Task};
use tui_input::Input;

/// Represents the current state of the application
pub struct App {
    /// Input field for entering commands
    pub input: Input,
    /// List of all tasks
    pub tasks: Vec<Task>,
    /// Next available task ID
    pub next_id: u32,
    /// Path to the tasks file
    pub tasks_file: String,
    /// State of the taskpad display
    pub taskpad_state: TaskpadState,
    /// Log of recent activity
    pub activity_log: ActivityLog,
    /// Whether to show help text
    pub show_help: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Creates a new application state
    #[must_use]
    pub fn new() -> Self {
        Self {
            input: Input::default(),
            tasks: Vec::new(),
            next_id: 1,
            tasks_file: "tasks.json".to_string(),
            taskpad_state: TaskpadState::new(),
            activity_log: ActivityLog::new(),
            show_help: true,
        }
    }

    /// Logs an activity message
    pub fn log_activity(&mut self, message: String) {
        self.activity_log.add_message(message);
    }
}

/// Runs the application, setting up the terminal,
/// loading the initial state from disk if available,
/// and processes user input until exit.
fn main() -> io::Result<()> {
    // Set up terminal
    let mut terminal = ui::setup_terminal()?;

    // Create app state
    let mut app = App::new();

    // Load tasks from disk if available
    if let Ok(tasks) = load_tasks(&app.tasks_file) {
        app.tasks = tasks;
        app.next_id = app.tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        app.taskpad_state.update_display_order(&app.tasks);
    }

    // Run app
    run_app(&mut terminal, app)?;

    // Restore terminal
    ui::restore_terminal(&mut terminal)?;
    Ok(())
}

/// Manages the main event loop of the application.
///
/// This function will loop until it sees an escape key or control-c.
/// It will then return, and the application will exit.
fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if event::poll(Duration::from_millis(100))? {
            let event = event::read()?;
            if let Event::Key(key) = event {
                if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL
                    || key.code == KeyCode::Esc
                {
                    return Ok(());
                }
                commands::handle_input_event(&mut app, event);
            }
        }
    }
}
