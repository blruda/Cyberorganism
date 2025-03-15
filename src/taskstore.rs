#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::wildcard_imports)]

//! Core data structures and persistence layer for cyberorganism. Handles task
//! representation, serialization, and file-based storage operations.

use chrono::{DateTime, Utc};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
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
    /// ID of the parent task if this is a subtask
    pub parent_id: Option<u32>,
    /// IDs of any subtasks this task has
    pub child_ids: Vec<u32>,
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
            parent_id: None,
            child_ids: Vec::new(),
        }
    }

    /// Updates the task's content
    pub fn update_content(&mut self, content: String) {
        self.content = content;
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

    /// Adds a subtask to this task, setting up the parent-child relationship
    pub fn add_subtask(&mut self, subtask_id: u32) {
        self.child_ids.push(subtask_id);
    }
}

/// Represents where the task is located in our system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskContainer {
    /// Task is ready to be processed
    Taskpad,
    /// Task has been organized into a project/area
    Backburner,
    /// Task is in the someday/maybe list
    Shelved,
    /// Task is archived
    Archived,
}

impl TaskContainer {
    /// Returns a human-readable name for the container
    pub const fn display_name(self) -> &'static str {
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

/// Low-level task operations that don't update display state
pub mod operations {
    use super::*;

    /// Remove a task from the task list (does not update display)
    pub fn remove_task(tasks: &mut Vec<Task>, index: usize) {
        tasks.remove(index);
    }

    /// Add a task to the task list (does not update display)
    pub fn add_task(tasks: &mut Vec<Task>, task: Task) {
        tasks.push(task);
    }

    /// Update a task in the task list (does not update display)
    pub fn update_task(tasks: &mut [Task], index: usize, update_fn: impl FnOnce(&mut Task)) {
        update_fn(&mut tasks[index]);
    }

    /// Remove a child from a parent task (does not update display)
    pub fn remove_child_from_parent(tasks: &mut [Task], parent_index: usize, child_id: u32) {
        if let Some(child_index) = tasks[parent_index]
            .child_ids
            .iter()
            .position(|&id| id == child_id)
        {
            tasks[parent_index].child_ids.remove(child_index);
        }
    }

    /// Find the nearest sibling of a task
    /// 
    /// Returns the ID of the nearest sibling task, preferring siblings above the current task
    /// If no siblings are found, returns None
    pub fn find_nearest_sibling(tasks: &[Task], task_id: u32) -> Option<u32> {
        // Find the task
        let task = tasks.iter().find(|t| t.id == task_id)?;
        
        // If it's a top-level task, return None
        let parent_id = task.parent_id?;
        
        // Find the parent task
        let parent = tasks.iter().find(|t| t.id == parent_id)?;
        
        // Get the index of the current task in the parent's child_ids
        let current_index = parent.child_ids.iter().position(|&id| id == task_id)?;
        
        // Try to find a sibling above first
        if current_index > 0 {
            return Some(parent.child_ids[current_index - 1]);
        }
        
        // If no sibling above, try to find a sibling below
        if current_index + 1 < parent.child_ids.len() {
            return Some(parent.child_ids[current_index + 1]);
        }
        
        // No siblings found
        None
    }
}

/// Finds a task in a slice of tasks by its ID
pub fn find_task_by_id(tasks: &[Task], id: u32) -> Option<usize> {
    tasks.iter().position(|task| task.id == id)
}

/// Finds a task in a slice of tasks by matching its content.
/// Prioritizes tasks in the active container over other tasks.
///
/// The matching is intentionally strict:
/// - Returns None for empty queries
/// - Only matches full content with tolerance for typos
/// - Case insensitive
#[allow(clippy::cast_possible_wrap)]
pub fn find_task_by_content(
    tasks: &[Task],
    query: &str,
    active_container: TaskContainer,
) -> Option<usize> {
    // Return None for empty queries
    if query.is_empty() {
        return None;
    }

    let matcher = SkimMatcherV2::default().ignore_case();

    // Calculate minimum score based on query length - allow roughly 1-2 typos
    let min_score = query.len() as i64 * 2 - 3;

    // First try tasks in active container
    let active_match = tasks
        .iter()
        .enumerate()
        .filter(|(_, task)| task.container == active_container)
        .filter_map(|(i, task)| {
            // We want the query length to be close to the task content length
            let len_diff = (task.content.len() as i64 - query.len() as i64).abs();
            if len_diff > 3 {
                // Allow for small differences in length
                return None;
            }

            matcher
                .fuzzy_match(&task.content, query)
                .filter(|&score| score >= min_score)
                .map(|score| (i, score))
        })
        .max_by_key(|(_, score)| *score)
        .map(|(i, _)| i);

    if active_match.is_some() {
        return active_match;
    }

    // If no match in active container, try all tasks
    tasks
        .iter()
        .enumerate()
        .filter_map(|(i, task)| {
            // We want the query length to be close to the task content length
            let len_diff = (task.content.len() as i64 - query.len() as i64).abs();
            if len_diff > 3 {
                // Allow for small differences in length
                return None;
            }

            matcher
                .fuzzy_match(&task.content, query)
                .filter(|&score| score >= min_score)
                .map(|score| (i, score))
        })
        .max_by_key(|(_, score)| *score)
        .map(|(i, _)| i)
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

#[cfg(test)]
pub struct TaskBuilder {
    task: Task,
}

#[cfg(test)]
impl TaskBuilder {
    pub fn new(id: u32) -> Self {
        Self {
            task: Task {
                id,
                content: format!("Task {}", id),
                created_at: Utc::now(),
                container: TaskContainer::Taskpad,
                status: TaskStatus::Todo,
                parent_id: None,
                child_ids: Vec::new(),
            },
        }
    }

    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.task.content = content.into();
        self
    }

    pub fn container(mut self, container: TaskContainer) -> Self {
        self.task.container = container;
        self
    }

    #[allow(dead_code)]
    pub fn parent(mut self, parent_id: u32) -> Self {
        self.task.parent_id = Some(parent_id);
        self
    }

    #[allow(dead_code)]
    pub fn children(mut self, child_ids: Vec<u32>) -> Self {
        self.task.child_ids = child_ids;
        self
    }

    pub fn build(self) -> Task {
        self.task
    }
}

#[cfg(test)]
mod tests {
    #[allow(clippy::wildcard_imports)]
    use super::*;
    use tempfile::tempdir;

    fn setup_test_tasks() -> Vec<Task> {
        vec![
            TaskBuilder::new(1).content("Buy groceries").build(),
            TaskBuilder::new(2).content("Call dentist").build(),
            TaskBuilder::new(3).content("Write report").build(),
        ]
    }

    #[test]
    fn test_task_new() {
        let task = Task::new(1, "Test task".to_string());
        assert_eq!(task.id, 1);
        assert_eq!(task.content, "Test task");
        assert!(matches!(task.container, TaskContainer::Taskpad));
        assert!(matches!(task.status, TaskStatus::Todo));
        assert!(task.parent_id.is_none());
        assert!(task.child_ids.is_empty());
    }

    #[test]
    fn test_task_complete() {
        let mut task = Task::new(1, "Test task".to_string());
        task.complete();
        assert!(matches!(task.status, TaskStatus::Done));
        assert!(matches!(task.container, TaskContainer::Archived));
    }

    #[test]
    fn test_task_container_methods() {
        let task = Task::new(1, "Test task".to_string());
        assert!(task.is_in_taskpad());
        assert_eq!(task.container(), &TaskContainer::Taskpad);
    }

    #[test]
    fn test_task_container_display_names() {
        assert_eq!(TaskContainer::Taskpad.display_name(), "taskpad");
        assert_eq!(TaskContainer::Backburner.display_name(), "backburner");
        assert_eq!(TaskContainer::Shelved.display_name(), "shelved");
        assert_eq!(TaskContainer::Archived.display_name(), "archived");
    }

    #[test]
    fn test_find_task_by_id() {
        let tasks = setup_test_tasks();
        assert_eq!(find_task_by_id(&tasks, 1), Some(0));
        assert_eq!(find_task_by_id(&tasks, 2), Some(1));
        assert_eq!(find_task_by_id(&tasks, 99), None);
    }

    #[test]
    fn test_find_task_by_id_with_multiple_tasks() {
        let mut tasks = vec![];
        operations::add_task(
            &mut tasks,
            TaskBuilder::new(4).content("Important meeting").build(),
        );
        let archived_task = TaskBuilder::new(5)
            .content("Archived task")
            .container(TaskContainer::Archived)
            .build();
        operations::add_task(&mut tasks, archived_task);

        assert_eq!(find_task_by_id(&tasks, 4), Some(0));
        assert_eq!(find_task_by_id(&tasks, 5), Some(1));
        assert_eq!(find_task_by_id(&tasks, 6), None);
    }

    #[test]
    fn test_find_task_by_content_case_insensitive() {
        let tasks = setup_test_tasks();
        // Should match exact content with different case
        assert!(find_task_by_content(&tasks, "BUY GROCERIES", TaskContainer::Taskpad).is_some());
        assert!(find_task_by_content(&tasks, "CALL DENTIST", TaskContainer::Taskpad).is_some());
    }

    #[test]
    fn test_find_task_by_content_empty_query() {
        let tasks = setup_test_tasks();
        assert!(find_task_by_content(&tasks, "", TaskContainer::Taskpad).is_none());
    }

    #[test]
    fn test_find_task_by_content_prioritizes_active_container() {
        let mut tasks = setup_test_tasks();
        // Create two tasks with similar content, one in taskpad and one archived
        tasks.push(TaskBuilder::new(4).content("Important meeting").build());
        let mut archived_task = TaskBuilder::new(5).content("Important meeting").build(); // Exact same content
        archived_task.complete(); // This moves it to archived
        tasks.push(archived_task);

        // Should find the taskpad task first
        let found_idx =
            find_task_by_content(&tasks, "Important meeting", TaskContainer::Taskpad).unwrap();
        assert_eq!(tasks[found_idx].id, 4);
        assert!(tasks[found_idx].is_in_taskpad());
    }

    #[test]
    fn test_save_and_load_tasks() -> std::io::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("tasks.json");
        let tasks = setup_test_tasks();

        // Test saving
        save_tasks(&tasks, file_path.to_str().unwrap())?;

        // Test loading
        let loaded_tasks = load_tasks(file_path.to_str().unwrap())?;
        assert_eq!(loaded_tasks.len(), tasks.len());
        assert_eq!(loaded_tasks[0].id, tasks[0].id);
        assert_eq!(loaded_tasks[0].content, tasks[0].content);

        Ok(())
    }

    #[test]
    fn test_load_tasks_nonexistent_file() -> std::io::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("nonexistent.json");

        let tasks = load_tasks(file_path.to_str().unwrap())?;
        assert!(tasks.is_empty());

        Ok(())
    }
}
