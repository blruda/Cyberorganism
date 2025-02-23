//! Core data structures and persistence layer for cyberorganism. Handles task
//! representation, serialization, and file-based storage operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
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

impl Task {
    /// Creates a new task with the given content
    pub fn new(id: u32, content: String) -> Self {
        Self {
            id,
            content,
            created_at: Utc::now(),
            container: TaskContainer::Taskpad,
            status: TaskStatus::Todo,
        }
    }

    /// Marks the task as complete and moves it to the archived container
    pub fn complete(&mut self) {
        self.status = TaskStatus::Done;
        self.container = TaskContainer::Archived;
    }

    /// Returns true if the task is in the taskpad container
    pub const fn is_in_taskpad(&self) -> bool {
        matches!(self.container, TaskContainer::Taskpad)
    }

    /// Returns the container this task is in
    pub const fn container(&self) -> &TaskContainer {
        &self.container
    }
}

/// Represents where the task is located in our system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl TaskContainer {
    /// Returns a human-readable name for the container
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Taskpad => "taskpad",
            Self::Backburner => "backburner",
            Self::Shelved => "shelved",
            Self::Archived => "archived",
        }
    }
}

/// Represents the current state of a task
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is new and needs attention
    Todo,
    /// Task is currently being worked on
    Doing,
    /// Task has been completed
    Done,
}

/// Finds a task in a slice of tasks by its ID
pub fn find_task_by_id(tasks: &[Task], id: u32) -> Option<usize> {
    tasks.iter().position(|task| task.id == id)
}

/// Finds a task in a slice of tasks by fuzzy matching its content.
/// Prioritizes tasks in the taskpad container over archived tasks.
pub fn find_task_by_content(tasks: &[Task], query: &str) -> Option<usize> {
    use fuzzy_matcher::skim::SkimMatcherV2;
    use fuzzy_matcher::FuzzyMatcher;
    let matcher = SkimMatcherV2::default();

    // First try to find a match in taskpad tasks
    let taskpad_match = tasks
        .iter()
        .enumerate()
        .filter(|(_, task)| task.is_in_taskpad())
        .max_by_key(|(_, task)| matcher.fuzzy_match(&task.content, query).unwrap_or(0));

    if let Some((index, task)) = taskpad_match {
        // Only return a match if it actually matches
        if let Some(_) = matcher.fuzzy_match(&task.content, query) {
            return Some(index);
        }
    }

    // If no taskpad match, look in all tasks
    tasks
        .iter()
        .enumerate()
        .max_by_key(|(_, task)| matcher.fuzzy_match(&task.content, query).unwrap_or(0))
        .and_then(|(index, task)| {
            // Only return a match if it actually matches
            if let Some(_) = matcher.fuzzy_match(&task.content, query) {
                Some(index)
            } else {
                None
            }
        })
}

/// Saves the current tasks to a JSON file.
///
/// ### Arguments
/// * `tasks` - A slice of tasks to save
/// * `path` - Path to the tasks storage file
pub fn save_tasks(tasks: &[Task], path: &str) -> std::io::Result<()> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(tasks)?;
    fs::write(path, json)
}

/// Loads tasks from the JSON file.
///
/// ### Arguments
/// * `path` - Path to the tasks storage file
pub fn load_tasks(path: &str) -> std::io::Result<Vec<Task>> {
    if Path::new(path).exists() {
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    } else {
        Ok(Vec::new())
    }
}
