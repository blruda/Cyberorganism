//! Main entry point for cyberorganism. This module coordinates
//! the application's state and event loop, delegating UI rendering, task operations, and
//! command handling to their respective specialized modules.

mod commands;
mod debug;
mod display_container;
mod keyhandler;
mod rendering;
mod taskstore;

use crate::display_container::{ActivityLog, DisplayContainerState};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::Terminal;
use std::io;
use std::time::Duration;
use taskstore::{Task, load_tasks};

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
        self.display_container_state
            .update_display_order(&self.tasks);
    }

    /// Add a task to the task list and update display order
    pub fn add_task(&mut self, task: Task) {
        taskstore::operations::add_task(&mut self.tasks, task);
        self.display_container_state
            .update_display_order(&self.tasks);
    }

    /// Update a task in the task list and update display order
    pub fn update_task<F>(&mut self, index: usize, update_fn: F)
    where
        F: FnOnce(&mut Task),
    {
        taskstore::operations::update_task(&mut self.tasks, index, update_fn);
        self.display_container_state
            .update_display_order(&self.tasks);
    }

    /// Remove a child from a parent task and update display order
    pub fn remove_child_from_parent(&mut self, parent_index: usize, child_id: u32) {
        taskstore::operations::remove_child_from_parent(&mut self.tasks, parent_index, child_id);
        self.display_container_state
            .update_display_order(&self.tasks);
    }
}

/// Runs the application, setting up the terminal,
/// loading the initial state from disk if available,
/// and processes user input until exit.
/// 
/// TODO: GUI REFACTOR - This function will need to be modified to support
/// a GUI implementation by abstracting the UI initialization and event loop.
fn main() -> io::Result<()> {
    // TODO: GUI REFACTOR - This TUI-specific setup will need to be replaced or abstracted
    // for GUI implementation
    let mut terminal = rendering::setup_terminal()?;

    // Create app state
    let mut app = App::new();

    // Load tasks from disk if available
    if let Ok(tasks) = load_tasks(&app.tasks_file) {
        app.tasks = tasks;
        app.next_id = app.tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        app.display_container_state.update_display_order(&app.tasks);
    }

    // TODO: GUI REFACTOR - This will need to be abstracted to support both TUI and GUI
    // implementations, possibly with a UI trait or factory pattern
    run_app(&mut terminal, app)?;

    // TODO: GUI REFACTOR - TUI-specific cleanup that will need to be abstracted
    rendering::restore_terminal(&mut terminal)?;
    Ok(())
}

/// Manages the main event loop of the application.
///
/// This function will loop until it sees an escape key or control-c.
/// It will then return, and the application will exit.
/// 
/// TODO: GUI REFACTOR - This function contains TUI-specific event handling and rendering.
/// It should be refactored to use an abstract UI interface that can be implemented
/// by both TUI and GUI backends.
fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> io::Result<()> {
    // TODO: GUI REFACTOR - TUI-specific input handling that will need to be abstracted
    // or replaced for GUI implementation
    let mut key_tracker = keyhandler::KeyCombinationTracker::new(100);

    // Use a moderate polling timeout to balance responsiveness and stability
    let polling_timeout = Duration::from_millis(33); // ~30 fps

    loop {
        // TODO: GUI REFACTOR - TUI-specific rendering that will need to be abstracted
        // for GUI implementation, possibly with a Renderer trait
        terminal.draw(|f| rendering::draw(f, &app))?;

        // Check for device_query key combinations
        let combination = key_tracker.check_combinations();
        if keyhandler::handle_key_combination(&mut app, combination) {
            // Don't immediately continue - instead, proceed to the event polling
            // This prevents bypassing the debounce mechanism
        }

        // TODO: GUI REFACTOR - TUI-specific event handling that will need to be abstracted
        // for GUI implementation, possibly with an EventHandler trait
        if event::poll(polling_timeout)? {
            let event = event::read()?;
            if let Event::Key(key) = event {
                if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL
                    || key.code == KeyCode::Esc
                {
                    return Ok(());
                }
                keyhandler::handle_input_event(&mut app, event, &key_tracker);
            }
        }
    }
}
