//! Main entry point for cyberorganism. This module coordinates
//! the application's state and event loop, delegating UI rendering, task operations, and
//! command handling to their respective specialized modules.

mod commands;
mod debug;
mod taskstore;
mod ui;

use crate::ui::{ActivityLog, DisplayContainerState};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::Terminal;
use std::io;
use std::time::Duration;
use taskstore::{load_tasks, Task};

/// Represents the current state of the application
pub struct App {
    /// List of all tasks
    pub tasks: Vec<Task>,
    /// Next available task ID
    pub next_id: u32,
    /// Path to the tasks file
    pub tasks_file: String,
    /// State of the taskpad display
    pub display_container_state: DisplayContainerState,
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
            tasks: Vec::new(),
            next_id: 1,
            tasks_file: "tasks.json".to_string(),
            display_container_state: DisplayContainerState::new(),
            activity_log: ActivityLog::new(),
            show_help: true,
        }
    }

    /// Logs an activity message
    pub fn log_activity(&mut self, message: String) {
        self.activity_log.add_message(message);
    }

    /// Remove a task from the task list and update display order
    pub fn remove_task(&mut self, index: usize) {
        taskstore::operations::remove_task(&mut self.tasks, index);
        self.display_container_state.update_display_order(&self.tasks);
    }

    /// Add a task to the task list and update display order
    pub fn add_task(&mut self, task: Task) {
        taskstore::operations::add_task(&mut self.tasks, task);
        self.display_container_state.update_display_order(&self.tasks);
    }

    /// Update a task in the task list and update display order
    pub fn update_task<F>(&mut self, index: usize, update_fn: F) 
    where F: FnOnce(&mut Task) {
        taskstore::operations::update_task(&mut self.tasks, index, update_fn);
        self.display_container_state.update_display_order(&self.tasks);
    }

    /// Remove a child from a parent task and update display order
    pub fn remove_child_from_parent(&mut self, parent_index: usize, child_id: u32) {
        taskstore::operations::remove_child_from_parent(&mut self.tasks, parent_index, child_id);
        self.display_container_state.update_display_order(&self.tasks);
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
        app.display_container_state.update_display_order(&app.tasks);
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
