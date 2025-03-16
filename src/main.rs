//! Main entry point for cyberorganism. This module coordinates
//! the application's state and event loop, delegating UI rendering, task operations, and
//! command handling to their respective specialized modules.

mod commands;
mod debug;
mod display_container;
mod genius_platform;
mod gui;
mod taskstore;

use crate::display_container::{ActivityLog, DisplayContainerState};
use std::fmt;
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

// Create a wrapper for eframe::Error that implements Send and Sync
#[derive(Debug)]
struct AppError(String);

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Application error: {}", self.0)
    }
}

impl std::error::Error for AppError {}

/// Runs the application, loading the initial state from disk if available,
/// and starting the GUI.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create app state
    let mut app = App::new();

    // Load tasks from disk if available
    if let Ok(tasks) = load_tasks(&app.tasks_file) {
        app.tasks = tasks;
        app.next_id = app.tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        app.display_container_state.update_display_order(&app.tasks);
    }

    // Initialize the Genius API from environment variables
    if genius_platform::initialize_from_env() {
        println!("Genius API initialized from environment variables");
    } else {
        println!("Genius API not configured. Set GENIUS_API_KEY and GENIUS_ORGANIZATION_ID environment variables to enable API integration.");
        // You could also load from a config file here as a fallback
    }

    // Run the GUI application
    if let Err(e) = gui::run_app(app) {
        eprintln!("Error running application: {}", e);
        return Err(Box::new(AppError(e.to_string())));
    }

    Ok(())
}

// Note: The run_app function has been moved to the gui module
