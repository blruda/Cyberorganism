//! Core data structures and persistence layer for cyberorganism. Handles task
//! representation, serialization, and file-based storage operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

/// A single task in the cyberorganism system.
///
/// Each task has a unique identifier, content, creation timestamp,
/// and tracks both its container and status.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    /// Unique identifier for the task
    pub id: u32,
    /// The actual task content
    pub content: String,
    /// When the task was created
    pub created_at: DateTime<Utc>,
    /// Current container of the task
    pub container: TaskContainer,
    /// Current status of the task
    pub status: TaskStatus,
}

/// Represents where the task is located in our system
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TaskContainer {
    /// Task is in the inbox, waiting to be processed
    Taskpad,
    /// Task has been organized into a project
    Backburner,
    /// Task is in the someday/maybe list
    Shelved,
    /// Task is archived
    Archived,
}

/// Represents the current state of a task
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TaskStatus {
    /// Task is new and needs attention
    Todo,
    /// Task is currently being worked on
    Doing,
    /// Task has been completed
    Done,
}

/// Saves the current tasks to a JSON file.
///
/// ### Arguments
/// * `tasks` - Vector of tasks to save
///
/// ### Returns
/// * `Ok(())` if save was successful
/// * `Err(io::Error)` if save failed
pub fn save_tasks(tasks: &[Task]) -> io::Result<()> {
    let json = serde_json::to_string_pretty(tasks)?;
    fs::write("tasks.json", json)?;
    Ok(())
}

/// Loads tasks from the JSON file.
///
/// ### Returns
/// * `Ok(Vec<Task>)` if load was successful
/// * `Err(io::Error)` if load failed or file doesn't exist
pub fn load_tasks() -> io::Result<Vec<Task>> {
    if Path::new("tasks.json").exists() {
        let json = fs::read_to_string("tasks.json")?;
        Ok(serde_json::from_str(&json)?)
    } else {
        Ok(Vec::new())
    }
}
