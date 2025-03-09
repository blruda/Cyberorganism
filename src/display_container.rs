#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::items_after_statements)]

//! Terminal user interface implementation using ratatui. Manages terminal setup,
//! teardown, and rendering of the task management interface.

use std::fmt;

use crate::taskstore::Task;

/// Manages the display state of tasks in the taskpad.
/// Tasks are displayed as a numbered list (1. Task A, 2. Task B, etc.)
/// with each task truncated to fit within a single line if necessary.
///
/// IMPORTANT: The `display_to_id` list contains only visible tasks based on the current
/// folding state. Whenever the folding state (`folded_tasks`) is modified,
/// `update_display_order` must be called to ensure the display list is synchronized.
#[derive(Debug)]
pub struct DisplayContainerState {
    /// List of task IDs for top-level tasks in the current container.
    /// This is used primarily for UI display purposes (e.g., focus navigation)
    /// and should NOT be used for task lookup - use `get_task_id_by_path` instead.
    pub display_to_id: Vec<u32>,
    /// Currently focused task index (0-based)
    pub focused_index: Option<usize>,
    /// Stores the original task focus index when creating a subtask
    /// Used to return focus after subtask creation
    pub original_focus: Option<usize>,
    /// Input field for entering commands
    input_value: String,
    /// Cursor position in the input field
    input_cursor: usize,
    /// Currently active container being displayed
    pub active_container: crate::taskstore::TaskContainer,
    /// Set of task IDs that are folded (not showing their children)
    pub folded_tasks: std::collections::HashSet<u32>,
}

impl Default for DisplayContainerState {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplayContainerState {
    pub fn new() -> Self {
        Self {
            display_to_id: Vec::new(),
            focused_index: Some(0), // Start focused on "Create new task or enter commands"
            original_focus: None,
            input_value: String::new(),
            input_cursor: 0,
            active_container: crate::taskstore::TaskContainer::Taskpad,
            folded_tasks: std::collections::HashSet::new(),
        }
    }

    /// Updates the display order based on the current tasks.
    /// Only includes tasks in the taskpad container (not archived).
    /// The display will show tasks as a numbered list starting from 1,
    /// with a special "Create new task or enter commands" entry at index 0.
    /// For tasks with subtasks:
    /// - Only top-level tasks are shown by default
    /// - Subtasks are shown only when their parent is expanded
    pub fn update_display_order(&mut self, tasks: &[Task]) {
        // First, collect all top-level tasks
        let mut display_ids = Vec::new();
        for task in tasks
            .iter()
            .filter(|t| t.container == self.active_container)
        {
            // Only include top-level tasks
            if task.parent_id.is_none() {
                display_ids.push(task.id);

                // If this task is expanded, add its children
                if self.is_task_expanded(task.id) {
                    // Add all children recursively
                    self.add_children_recursively(task.id, tasks, &mut display_ids);
                }
            }
        }

        self.display_to_id = display_ids;

        // Reset focus to 0 if it's beyond the new list length
        if let Some(current) = self.focused_index {
            if current > self.display_to_id.len() {
                self.focused_index = Some(0);
                self.update_input_for_focus(tasks);
            }
        }

        // Always sync input with current focus since the task at each index might have changed
        self.update_input_for_focus(tasks);
    }

    /// Helper function to recursively add children of a task to the display order
    fn add_children_recursively(&self, parent_id: u32, tasks: &[Task], display_ids: &mut Vec<u32>) {
        if let Some(parent_task) = tasks.iter().find(|t| t.id == parent_id) {
            for child_id in &parent_task.child_ids {
                display_ids.push(*child_id);
                // If this child is also expanded, add its children too
                if self.is_task_expanded(*child_id) {
                    self.add_children_recursively(*child_id, tasks, display_ids);
                }
            }
        }
    }

    /// Gets a task ID from a hierarchical display index like "1.2.3"
    /// The display index represents the visual position in the UI, where:
    /// - "1" means the first top-level task
    /// - "1.2" means the second child of the first top-level task
    pub fn get_task_id_by_path(&self, display_path_str: &str, tasks: &[Task]) -> Option<u32> {
        // Parse the display path (e.g., "1.2.3" -> [1,2,3])
        let display_path = TaskIndex::from_str(display_path_str).ok()?;
        let path = display_path.path();

        // Get all visible top-level tasks
        let visible_tasks: Vec<&Task> = tasks
            .iter()
            .filter(|t| t.container == self.active_container && t.parent_id.is_none())
            .collect();

        // Get the first task using the first index (1-based)
        let first_pos = path[0].checked_sub(1)?;
        let mut current_task = *visible_tasks.get(first_pos)?;

        // For each subsequent index in the path, find the child at that position
        for &child_display_pos in &path[1..] {
            // Only proceed if the current task is expanded
            if !self.is_task_expanded(current_task.id) {
                return None;
            }

            let child_pos = child_display_pos.checked_sub(1)?;

            let visible_children: Vec<&Task> = current_task
                .child_ids
                .iter()
                .filter_map(|&id| tasks.iter().find(|t| t.id == id))
                .collect();

            current_task = *visible_children.get(child_pos)?;
        }

        Some(current_task.id)
    }

    /// Gets the display index (1-based) for a task ID
    pub fn get_display_index(&self, task_id: u32) -> Option<usize> {
        self.display_to_id
            .iter()
            .position(|&id| id == task_id)
            .map(|i| i + 1)
    }

    /// Returns the number of tasks in the display (excluding "Create new task or enter commands" entry)
    pub fn len(&self) -> usize {
        self.display_to_id.len()
    }

    /// Returns true if there are no tasks in the display (may still have "Create new task or enter commands" entry)
    pub fn is_empty(&self) -> bool {
        self.display_to_id.is_empty()
    }

    /// Focus the previous task (move up), with wrapping
    pub fn focus_previous(&mut self) {
        let max_index = self.display_to_id.len();
        self.focused_index = Some(match self.focused_index {
            Some(0) => max_index, // Wrap to bottom
            Some(current) => current - 1,
            None => 0, // Start at "Create new task or enter commands"
        });
    }

    /// Focus the next task (move down), with wrapping
    pub fn focus_next(&mut self) {
        let max_index = self.display_to_id.len();
        self.focused_index = Some(match self.focused_index {
            Some(current) if current >= max_index => 0, // Wrap to top
            Some(current) => current + 1,
            None => 0, // Start at "Create new task or enter commands"
        });
    }

    /// Clear the current focus
    pub fn clear_focus(&mut self) {
        self.focused_index = None;
    }

    /// Gets the content of the currently focused task.
    /// Returns None if no task is focused or if the focused item is the "Create new task or enter commands" entry.
    pub fn get_focused_task_content<'a>(&self, tasks: &'a [Task]) -> Option<&'a str> {
        match self.focused_index {
            Some(0) => None, // "Create new task or enter commands" entry
            Some(idx) if idx <= self.display_to_id.len() => {
                let task_id = self.display_to_id[idx - 1];
                tasks
                    .iter()
                    .find(|task| task.id == task_id)
                    .map(|task| task.content.as_str())
            }
            _ => None,
        }
    }

    // Input buffer methods - no event handling, just state management
    pub fn input_value(&self) -> &str {
        &self.input_value
    }

    pub fn input_cursor(&self) -> usize {
        self.input_cursor
    }

    pub fn reset_input(&mut self) {
        self.input_value.clear();
        self.input_cursor = 0;
    }

    pub fn set_input(&mut self, content: &str) {
        self.input_value = content.to_string();
        self.input_cursor = content.len();
    }

    pub fn update_input_for_focus(&mut self, tasks: &[Task]) {
        // If there are no tasks in the current container, reset focus to 0 and clear input
        let has_tasks_in_container = tasks.iter().any(|t| t.container == self.active_container);
        if !has_tasks_in_container {
            self.focused_index = Some(0);
            self.reset_input();
            return;
        }

        match self.focused_index {
            Some(0) => self.reset_input(),
            _ => {
                if let Some(content) = self.get_focused_task_content(tasks) {
                    self.set_input(&content);
                } else {
                    // If focused task doesn't exist anymore, reset to 0
                    self.focused_index = Some(0);
                    self.reset_input();
                }
            }
        }
    }

    pub fn set_cursor_position(&mut self, position: usize) {
        self.input_cursor = position.min(self.input_value.len());
    }
    
    pub fn get_input_mut(&mut self) -> (&mut String, &mut usize) {
        (&mut self.input_value, &mut self.input_cursor)
    }

    /// Toggle the expansion state of a task and update the display list
    ///
    /// This ensures that the `display_to_id` list is always in sync with the folding state.
    pub fn toggle_task_expansion(&mut self, task_id: u32, tasks: &[Task]) {
        if self.folded_tasks.contains(&task_id) {
            self.folded_tasks.remove(&task_id);
        } else {
            self.folded_tasks.insert(task_id);
        }

        // Always update display order after changing folding state
        self.update_display_order(tasks);
    }

    /// Check if a task is expanded
    pub fn is_task_expanded(&self, task_id: u32) -> bool {
        !self.folded_tasks.contains(&task_id)
    }

    /// Collapse all tasks
    pub fn collapse_all(&mut self) {
        self.folded_tasks = self.display_to_id.iter().copied().collect();
    }

    /// Fold a specific task
    pub fn fold_task(&mut self, task_id: u32) {
        self.folded_tasks.insert(task_id);
    }

    /// Fold a list of tasks
    pub fn fold_tasks(&mut self, task_ids: &[u32]) {
        self.folded_tasks.extend(task_ids.iter().copied());
    }
}

/// Represents a hierarchical task index like "1.2.3"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskIndex {
    /// Path to the task, e.g. [1, 2, 3] for "1.2.3"
    pub path: Vec<usize>,
}

impl TaskIndex {
    /// Create a new `TaskIndex` from a string like "1.2.3"
    pub fn from_str(s: &str) -> Result<Self, String> {
        let path: Result<Vec<usize>, _> =
            s.trim_end_matches('.').split('.').map(str::parse).collect();

        match path {
            Ok(path) if path.is_empty() => Err("Empty task index".to_string()),
            Ok(path) if path.iter().any(|&x| x == 0) => {
                Err("Task indices must be positive".to_string())
            }
            Ok(path) => Ok(Self { path }),
            Err(_) => Err("Invalid task index format".to_string()),
        }
    }

    pub fn path(&self) -> &[usize] {
        &self.path
    }
}

impl fmt::Display for TaskIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.path
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(".")
        )
    }
}

/// Maintains a log of user activities and commands
#[derive(Default)]
pub struct ActivityLog {
    /// List of activity messages, newest first
    messages: Vec<String>,
}

impl ActivityLog {
    /// Creates a new empty activity log
    pub const fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// Adds a new activity message to the log
    pub fn add_message(&mut self, message: String) {
        self.messages.insert(0, message);
    }

    /// Gets the most recent activity message
    pub fn latest_message(&self) -> Option<&str> {
        self.messages.first().map(std::string::String::as_str)
    }
}
