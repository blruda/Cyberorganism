#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::wildcard_imports)]

//! Terminal user interface implementation using ratatui. Manages terminal setup,
//! teardown, and rendering of the task management interface.

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::io;
use tui_input::Input;

/// The primary accent color used throughout the UI
const ACCENT_COLOR: Color = Color::Rgb(57, 255, 20);

use crate::{
    taskstore::{Task, TaskContainer, TaskStatus},
    App,
};

/// Initializes the terminal for TUI operation.
///
/// ### Returns
/// * `Ok(Terminal)` - Configured terminal instance ready for TUI
/// * `Err(io::Error)` - If terminal setup fails
///
/// Sets up:
/// * Raw mode for immediate character input
/// * Alternate screen to preserve original terminal content
/// * Mouse capture for potential future mouse support
pub fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

/// Restores the terminal to its original state.
///
/// ### Returns
/// * `Ok(())` - Terminal successfully restored
/// * `Err(io::Error)` - If cleanup fails
pub fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Manages the display state of tasks in the taskpad.
/// Tasks are displayed as a numbered list (1. Task A, 2. Task B, etc.)
/// with each task truncated to fit within a single line if necessary.
#[derive(Debug)]
pub struct DisplayContainerState {
    /// List of task IDs for top-level tasks in the current container.
    /// This is used primarily for UI display purposes (e.g., focus navigation)
    /// and should NOT be used for task lookup - use get_task_id_by_path instead.
    pub display_to_id: Vec<u32>,
    /// Currently focused task index (0-based)
    pub focused_index: Option<usize>,
    /// Input field for entering commands
    input: Input,
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
            focused_index: Some(0), // Start focused on "Create new task"
            input: Input::default(),
            active_container: crate::taskstore::TaskContainer::Taskpad,
            folded_tasks: std::collections::HashSet::new(),
        }
    }

    /// Updates the display order based on the current tasks.
    /// Only includes tasks in the taskpad container (not archived).
    /// The display will show tasks as a numbered list starting from 1,
    /// with a special "Create new task" entry at index 0.
    /// For tasks with subtasks:
    /// - Only top-level tasks are shown by default
    /// - Subtasks are shown only when their parent is expanded
    pub fn update_display_order(&mut self, tasks: &[Task]) {
        use crate::debug::log_debug;
        log_debug(&format!(
            "Updating display for container: {:?}",
            self.active_container
        ));

        // First, collect all top-level tasks
        let mut display_ids = Vec::new();
        for task in tasks.iter().filter(|t| t.container == self.active_container) {
            // Only include top-level tasks
            if task.parent_id.is_none() {
                log_debug(&format!("Adding top-level task {} to display_to_id", task.id));
                display_ids.push(task.id);
                
                // If this task is expanded, add its children
                if self.is_task_expanded(task.id) {
                    // Add all children recursively
                    self.add_children_recursively(task.id, tasks, &mut display_ids);
                }
            }
        }
        
        log_debug(&format!("Final display_to_id: {:?}", display_ids));
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
        use crate::debug::log_debug;
        log_debug(&format!("Looking up task by display path: {}", display_path_str));
        
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
        log_debug(&format!("Found top-level task at position {}: {}", first_pos, current_task.id));
        
        // For each subsequent index in the path, find the child at that position
        for &child_display_pos in &path[1..] {
            // Only proceed if the current task is expanded
            if !self.is_task_expanded(current_task.id) {
                return None;
            }
            
            let child_pos = child_display_pos.checked_sub(1)?;
            
            let visible_children: Vec<&Task> = current_task.child_ids
                .iter()
                .filter_map(|&id| tasks.iter().find(|t| t.id == id))
                .collect();
            
            current_task = *visible_children.get(child_pos)?;
            log_debug(&format!("Found child at position {}: {}", child_pos, current_task.id));
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

    /// Returns the number of tasks in the display (excluding "Create new task" entry)
    pub fn len(&self) -> usize {
        self.display_to_id.len()
    }

    /// Returns true if there are no tasks in the display (may still have "Create new task" entry)
    pub fn is_empty(&self) -> bool {
        self.display_to_id.is_empty()
    }

    /// Focus the previous task (move up), with wrapping
    pub fn focus_previous(&mut self) {
        let max_index = self.display_to_id.len();
        self.focused_index = Some(match self.focused_index {
            Some(0) => max_index, // Wrap to bottom
            Some(current) => current - 1,
            None => 0, // Start at "Create new task"
        });
    }

    /// Focus the next task (move down), with wrapping
    pub fn focus_next(&mut self) {
        let max_index = self.display_to_id.len();
        self.focused_index = Some(match self.focused_index {
            Some(current) if current >= max_index => 0, // Wrap to top
            Some(current) => current + 1,
            None => 0, // Start at "Create new task"
        });
    }

    /// Clear the current focus
    pub fn clear_focus(&mut self) {
        self.focused_index = None;
    }

    /// Gets the content of the currently focused task.
    /// Returns None if no task is focused or if the focused item is the "Create new task" entry.
    pub fn get_focused_task_content<'a>(&self, tasks: &'a [Task]) -> Option<&'a str> {
        match self.focused_index {
            Some(0) => None, // "Create new task" entry
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
        self.input.value()
    }

    pub fn input_cursor(&self) -> usize {
        self.input.cursor()
    }

    pub fn reset_input(&mut self) {
        self.input.reset();
    }

    pub fn set_input(&mut self, content: &str) {
        self.input = Input::new(content.to_string());
    }

    pub fn update_input_for_focus(&mut self, tasks: &[Task]) {
        // If there are no tasks in the current container, reset focus to 0 and clear input
        let has_tasks_in_container = tasks.iter().any(|t| t.container == self.active_container);
        if !has_tasks_in_container {
            self.focused_index = Some(0);
            self.input.reset();
            return;
        }

        match self.focused_index {
            Some(0) => self.input.reset(),
            _ => {
                if let Some(content) = self.get_focused_task_content(tasks) {
                    self.input = Input::new(content.to_string());
                } else {
                    // If focused task doesn't exist anymore, reset to 0
                    self.focused_index = Some(0);
                    self.input.reset();
                }
            }
        }
    }

    pub fn get_input_mut(&mut self) -> &mut Input {
        &mut self.input
    }

    /// Toggle the expansion state of a task
    pub fn toggle_task_expansion(&mut self, task_id: u32) {
        if self.folded_tasks.contains(&task_id) {
            self.folded_tasks.remove(&task_id);
        } else {
            self.folded_tasks.insert(task_id);
        }
    }

    /// Check if a task is expanded
    pub fn is_task_expanded(&self, task_id: u32) -> bool {
        !self.folded_tasks.contains(&task_id)
    }

    /// Collapse all tasks
    pub fn collapse_all(&mut self) {
        self.folded_tasks = self.display_to_id.iter().cloned().collect();
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
#[derive(Debug, Clone, PartialEq)]
pub struct TaskIndex {
    /// Path to the task, e.g. [1, 2, 3] for "1.2.3"
    path: Vec<usize>,
}

impl TaskIndex {
    /// Create a new TaskIndex from a string like "1.2.3"
    pub fn from_str(s: &str) -> Result<Self, String> {
        let path: Result<Vec<usize>, _> = s
            .trim_end_matches('.')
            .split('.')
            .map(|part| part.parse::<usize>())
            .collect();
        
        match path {
            Ok(path) if path.is_empty() => Err("Empty task index".to_string()),
            Ok(path) if path.iter().any(|&x| x == 0) => Err("Task indices must be positive".to_string()),
            Ok(path) => Ok(TaskIndex { path }),
            Err(_) => Err("Invalid task index format".to_string()),
        }
    }

    /// Convert to string representation
    pub fn to_string(&self) -> String {
        self.path.iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(".")
    }

    pub fn path(&self) -> &[usize] {
        &self.path
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

/// Calculate the available width for content inside a frame area
fn calculate_available_width(frame_size: Rect) -> usize {
    let temp_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Taskpad
            Constraint::Length(3), // Temporary input height to get width (1 line + borders)
        ])
        .split(frame_size);

    temp_chunks[1].width.saturating_sub(2) as usize
}

/// Process input text into lines and calculate required height
fn calculate_input_dimensions(
    input_value: &str,
    cursor_position: usize,
    available_width: usize,
) -> (Vec<Line>, u16) {
    let needed_lines = ((cursor_position.max(input_value.len())) / available_width + 1).max(1);

    let mut lines: Vec<Line> = input_value
        .chars()
        .collect::<Vec<_>>()
        .chunks(available_width)
        .map(|chunk| Line::from(chunk.iter().collect::<String>()))
        .collect();

    // Ensure we have enough lines for the cursor
    while lines.len() < needed_lines {
        lines.push(Line::from(""));
    }

    let total_height = (lines.len() + 2) as u16; // +2 for borders
    (lines, total_height)
}

/// Create layout constraints based on UI state
fn create_layout_constraints(input_height: u16, show_help: bool) -> Vec<Constraint> {
    let mut constraints = vec![
        Constraint::Min(1),    // Taskpad - take remaining space
        Constraint::Length(1), // Activity log - single line
    ];

    // Only add help message constraint if it's visible
    if show_help {
        constraints.push(Constraint::Length(1)); // Help message - single line
    }

    // Add input constraint
    constraints.push(Constraint::Length(input_height)); // Input - exact height needed
    constraints
}

/// Format a single task line with index and truncation if needed
fn format_task_line<'a>(
    task_index: &TaskIndex,
    task: &'a Task,
    display_state: &'a DisplayContainerState,
    available_width: usize,
    is_focused: bool,
) -> Line<'a> {
    let mut spans = Vec::new();
    let depth = task_index.path().len() - 1; // Depth is one less than path length
    let indent = "  ".repeat(depth); // Two spaces per level

    // Add task index
    spans.push(Span::raw(format!("{}{}", indent, task_index.to_string())));
    spans.push(Span::raw(". "));

    // Add completion status indicator
    if task.status == TaskStatus::Done {
        spans.push(Span::styled("✓ ", Style::default().fg(ACCENT_COLOR)));
    }

    // Add expansion indicator if task has children
    if !task.child_ids.is_empty() {
        let indicator = if display_state.is_task_expanded(task.id) {
            "▼ "
        } else {
            "▶ "
        };
        spans.push(Span::raw(indicator.to_string()));
    }

    // Calculate remaining width for task content
    let prefix_width = indent.len() + task_index.to_string().len() + 2; // index + ". "
    let status_width = if task.status == TaskStatus::Done { 2 } else { 0 }; // "✓ "
    let indicator_width = if task.child_ids.is_empty() { 0 } else { 2 }; // "▼ " or "▶ "
    let content_width = available_width.saturating_sub(prefix_width + status_width + indicator_width);

    // Add task content
    let content = if task.content.len() > content_width {
        format!("{}...", &task.content[..content_width.saturating_sub(3)])
    } else {
        task.content.clone()
    };

    spans.push(if is_focused {
        Span::styled(content, Style::default().add_modifier(Modifier::REVERSED))
    } else {
        Span::raw(content)
    });

    Line::from(spans)
}

/// Create task lines for display, including subtasks
fn create_task_lines<'a>(
    tasks: &'a [Task],
    display_state: &'a DisplayContainerState,
    available_width: usize,
    focused_index: Option<usize>,
) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    let mut display_index = 1; // Track the current display index for focus

    // Add the "Create new task" entry at index 0
    let create_task_style = if focused_index == Some(0) {
        Style::default()
            .fg(Color::Black)
            .bg(ACCENT_COLOR)
    } else {
        Style::default().fg(ACCENT_COLOR)
    };
    lines.push(Line::from(vec![Span::styled(
        "<Create new task>",
        create_task_style,
    )]));

    // Helper function to recursively add tasks and their subtasks
    fn add_task_lines<'a>(
        task: &'a Task,
        current_index: &mut Vec<usize>,
        display_index: &mut usize,
        tasks: &'a [Task],
        display_state: &'a DisplayContainerState,
        available_width: usize,
        focused_index: Option<usize>,
        lines: &mut Vec<Line<'a>>,
    ) {
        // Create TaskIndex for current task
        let task_index = TaskIndex { path: current_index.clone() };
        
        // Add the current task
        lines.push(format_task_line(
            &task_index,
            task,
            display_state,
            available_width,
            Some(*display_index) == focused_index,
        ));
        *display_index += 1;

        // If task is expanded and has children, add them
        if display_state.is_task_expanded(task.id) {
            for (child_idx, child_id) in task.child_ids.iter().enumerate() {
                if let Some(child_task) = tasks.iter().find(|t| t.id == *child_id) {
                    current_index.push(child_idx + 1); // 1-based index
                    add_task_lines(
                        child_task,
                        current_index,
                        display_index,
                        tasks,
                        display_state,
                        available_width,
                        focused_index,
                        lines,
                    );
                    current_index.pop();
                }
            }
        }
    }

    // Add top-level tasks and their subtasks
    for (idx, task) in tasks
        .iter()
        .filter(|task| {
            task.container == display_state.active_container && task.parent_id.is_none()
        })
        .enumerate()
    {
        let mut current_index = vec![idx + 1]; // 1-based index
        add_task_lines(
            task,
            &mut current_index,
            &mut display_index,
            tasks,
            display_state,
            available_width,
            focused_index,
            &mut lines,
        );
    }

    lines
}

/// Create the tasks widget
fn create_tasks_widget<'a>(tasks_text: Vec<Line<'a>>, container_name: &'a str) -> Paragraph<'a> {
    Paragraph::new(tasks_text)
        .block(Block::default().borders(Borders::ALL).title(container_name))
        .style(Style::default().fg(ACCENT_COLOR))
        .wrap(Wrap { trim: false })
}

/// Create the activity log widget
fn create_activity_log_widget(message: &str) -> Paragraph<'_> {
    Paragraph::new(Line::from(vec![Span::styled(
        message,
        Style::default().fg(ACCENT_COLOR),
    )]))
}

/// Create the help widget
fn create_help_widget() -> Paragraph<'static> {
    Paragraph::new(vec![Line::from(vec![
        Span::styled(
            "Press ".to_string(),
            Style::default().fg(ACCENT_COLOR),
        ),
        Span::styled(
            "esc".to_string(),
            Style::default()
                .fg(ACCENT_COLOR)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " or ".to_string(),
            Style::default().fg(ACCENT_COLOR),
        ),
        Span::styled(
            "ctrl-c".to_string(),
            Style::default()
                .fg(ACCENT_COLOR)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " to exit cyberorganism".to_string(),
            Style::default().fg(ACCENT_COLOR),
        ),
    ])])
}

/// Create the input widget
fn create_input_widget(lines: Vec<Line<'_>>) -> Paragraph<'_> {
    Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(ACCENT_COLOR))
}

/// Calculate cursor position within the input area
const fn calculate_cursor_position(cursor_pos: usize, available_width: usize) -> (u16, u16) {
    let cursor_x = cursor_pos as u16 % available_width as u16;
    let cursor_y = (cursor_pos / available_width) as u16;
    (cursor_x, cursor_y)
}

/// Draws the application UI.
/// The main area shows tasks in the taskpad as a numbered list:
///   1. First task
///   2. Second task
///   3. Very long task that exceeds the width will be trunc...
///
/// Tasks are filtered to only show those in the taskpad (not archived),
/// and each task is truncated if it would exceed the width of the display.
pub fn draw(frame: &mut Frame, app: &App) {
    let available_width = calculate_available_width(frame.size());
    let (input_lines, input_height) = calculate_input_dimensions(
        app.display_container_state.input_value(),
        app.display_container_state.input_cursor(),
        available_width,
    );

    let constraints = create_layout_constraints(input_height, app.show_help);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(frame.size());

    // Render tasks
    let task_lines = create_task_lines(
        &app.tasks,
        &app.display_container_state,
        available_width,
        app.display_container_state.focused_index,
    );
    let tasks_widget = create_tasks_widget(
        task_lines,
        match app.display_container_state.active_container {
            TaskContainer::Taskpad => "Taskpad",
            TaskContainer::Backburner => "Backburner",
            TaskContainer::Shelved => "Shelved",
            TaskContainer::Archived => "Archived",
        },
    );
    frame.render_widget(tasks_widget, chunks[0]);

    // Render activity log if there's a message
    if let Some(message) = app.activity_log.latest_message() {
        let log_widget = create_activity_log_widget(message);
        frame.render_widget(log_widget, chunks[1]);
    }

    // Render help message if needed
    if app.show_help {
        let help_widget = create_help_widget();
        frame.render_widget(help_widget, chunks[2]);
    }

    // Render input
    let input_widget = create_input_widget(input_lines);
    let input_chunk = if app.show_help { chunks[3] } else { chunks[2] };
    frame.render_widget(input_widget, input_chunk);

    // Set cursor position
    let (cursor_x, cursor_y) =
        calculate_cursor_position(app.display_container_state.input_cursor(), available_width);
    frame.set_cursor(input_chunk.x + 1 + cursor_x, input_chunk.y + 1 + cursor_y);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taskstore::{Task, TaskContainer, TaskStatus};
    use chrono::Utc;

    pub struct TaskBuilder {
        id: u32,
        content: String,
        container: TaskContainer,
        created_at: chrono::DateTime<Utc>,
        status: TaskStatus,
        parent_id: Option<u32>,
        child_ids: Vec<u32>,
    }

    impl TaskBuilder {
        pub fn new(id: u32) -> Self {
            Self {
                id,
                content: String::new(),
                container: TaskContainer::Taskpad,
                created_at: Utc::now(),
                status: TaskStatus::Todo,
                parent_id: None,
                child_ids: Vec::new(),
            }
        }

        pub fn content(mut self, content: &str) -> Self {
            self.content = content.to_string();
            self
        }

        pub fn container(mut self, container: TaskContainer) -> Self {
            self.container = container;
            self
        }

        pub fn parent(mut self, parent_id: u32) -> Self {
            self.parent_id = Some(parent_id);
            self
        }

        pub fn children(mut self, child_ids: Vec<u32>) -> Self {
            self.child_ids = child_ids;
            self
        }

        pub fn build(self) -> Task {
            Task {
                id: self.id,
                content: self.content,
                container: self.container,
                created_at: self.created_at,
                status: self.status,
                parent_id: self.parent_id,
                child_ids: self.child_ids,
            }
        }
    }

    #[test]
    fn test_taskpad_display_order() {
        let mut state = DisplayContainerState::new();
        let tasks = vec![
            TaskBuilder::new(1).children(vec![3]).build(),
            TaskBuilder::new(2).container(TaskContainer::Archived).build(),
            TaskBuilder::new(3).content("Task 1.1").parent(1).build(),
        ];

        // Start with task 1 folded
        state.fold_task(1);
        state.update_display_order(&tasks);
        // println!("Initial display_to_id: {:?}", state.display_to_id);
        // println!("Initial len: {}", state.len());
        // println!("Task 1's children: {:?}", tasks[0].child_ids);
        // println!("Task 1 expanded? {}", state.is_task_expanded(1));
        assert_eq!(state.len(), 1);  // Only task 1 is visible
        assert_eq!(state.get_task_id_by_path("1", &tasks), Some(1));
        assert_eq!(state.get_task_id_by_path("1.1", &tasks), None);  // Not visible until parent is expanded

        // After expanding task 1, its child becomes visible
        state.toggle_task_expansion(1);
        // println!("\nAfter expansion:");
        // println!("Task 1 expanded? {}", state.is_task_expanded(1));
        state.update_display_order(&tasks);
        // println!("display_to_id after expansion: {:?}", state.display_to_id);
        // println!("len after expansion: {}", state.len());
        assert_eq!(state.len(), 2);  // Now both task 1 and task 3 are visible
        assert_eq!(state.get_task_id_by_path("1", &tasks), Some(1));
        assert_eq!(state.get_task_id_by_path("1.1", &tasks), Some(3));

        // Change container to archive
        state.active_container = TaskContainer::Archived;
        state.update_display_order(&tasks);
        // println!("\nAfter archive:");
        // println!("display_to_id in archive: {:?}", state.display_to_id);
        // println!("len in archive: {}", state.len());
        assert_eq!(state.len(), 1);  // Only task 2 is visible
        assert_eq!(state.get_task_id_by_path("1", &tasks), Some(2));
    }

    #[test]
    fn test_get_task_id_by_path() {
        let tasks = vec![
            TaskBuilder::new(7).children(vec![9]).build(),
            TaskBuilder::new(6).build(),
            TaskBuilder::new(9).content("Task 1.1").parent(7).build(),
        ];

        let mut display = DisplayContainerState::default();
        display.active_container = TaskContainer::Taskpad;
        
        // Test that we find tasks by their position in the hierarchy,
        // not by their position in display_to_id
        assert_eq!(display.get_task_id_by_path("1", &tasks), Some(7));   // First top-level task
        assert_eq!(display.get_task_id_by_path("2", &tasks), Some(6));   // Second top-level task
        assert_eq!(display.get_task_id_by_path("1.1", &tasks), Some(9)); // First subtask of first task
        
        // Test invalid paths
        assert_eq!(display.get_task_id_by_path("3", &tasks), None);      // Non-existent top-level task
        assert_eq!(display.get_task_id_by_path("1.2", &tasks), None);    // Non-existent subtask
        assert_eq!(display.get_task_id_by_path("2.1", &tasks), None);    // Subtask of task with no children
    }

    #[test]
    fn test_taskpad_empty() {
        let mut state = DisplayContainerState::new();
        let tasks = Vec::new();

        state.update_display_order(&tasks);
        assert!(state.is_empty());
        assert_eq!(state.get_task_id_by_path("1", &tasks), None);
    }

    #[test]
    fn test_activity_log() {
        let mut log = ActivityLog::new();

        // Test empty log
        assert_eq!(log.latest_message(), None);

        // Test single message
        log.add_message("First message".to_string());
        assert_eq!(log.latest_message(), Some("First message"));

        // Test message limit
        for i in 0..20 {
            log.add_message(format!("Message {}", i));
        }

        // Should keep the most recent message
        assert_eq!(log.latest_message(), Some("Message 19"));
    }

    #[test]
    fn test_calculate_input_dimensions() {
        // Test empty input
        let (lines, height) = calculate_input_dimensions("", 0, 10);
        assert_eq!(lines.len(), 1, "Empty input should have one line");
        assert_eq!(
            height, 3,
            "Empty input should have height of 3 (1 line + 2 borders)"
        );

        // Test single line input
        let (lines, height) = calculate_input_dimensions("hello", 5, 10);
        assert_eq!(lines.len(), 1, "Short input should fit in one line");
        assert_eq!(height, 3, "Single line should have height of 3");

        // Test input wrapping
        let (lines, height) = calculate_input_dimensions("hello world", 11, 5);
        assert_eq!(lines.len(), 3, "Should split into three lines");
        assert_eq!(height, 5, "Three lines plus borders should be height 5");

        // Test cursor beyond text
        let (lines, height) = calculate_input_dimensions("hi", 10, 5);
        assert_eq!(lines.len(), 3, "Should have enough lines for cursor");
        assert_eq!(height, 5, "Should have height for cursor position");
    }

    #[test]
    fn test_create_layout_constraints() {
        // Test without help message
        let constraints = create_layout_constraints(3, false);
        assert_eq!(
            constraints.len(),
            3,
            "Should have 3 constraints without help"
        );

        // Test with help message
        let constraints = create_layout_constraints(3, true);
        assert_eq!(constraints.len(), 4, "Should have 4 constraints with help");
    }

    #[test]
    fn test_format_task_line() {
        let now = Utc::now();
        let display_state = DisplayContainerState::new();

        // Test normal task
        let task = Task {
            id: 1,
            content: "Test task".to_string(),
            container: TaskContainer::Taskpad,
            created_at: now,
            status: TaskStatus::Todo,
            parent_id: None,
            child_ids: Vec::new(),
        };
        let line = format_task_line(
            &TaskIndex { path: vec![1] },
            &task,
            &display_state,
            20,
            false,
        );
        // Check that the spans together form "1. Test task"
        let full_content: String = line.spans.iter().map(|span| &span.content[..]).collect();
        assert!(full_content.starts_with("1. Test task"));

        // Test task truncation
        let long_task = Task {
            id: 2,
            content: "This is a very long task that should be truncated".to_string(),
            container: TaskContainer::Taskpad,
            created_at: now,
            status: TaskStatus::Todo,
            parent_id: None,
            child_ids: Vec::new(),
        };
        let line = format_task_line(
            &TaskIndex { path: vec![1] },
            &long_task,
            &display_state,
            20,
            false,
        );
        let full_content: String = line.spans.iter().map(|span| &span.content[..]).collect();
        assert!(
            full_content.len() <= 20,
            "Line should be truncated to width"
        );
        assert!(
            full_content.contains("..."),
            "Truncated line should end with ..."
        );

        // Test task with double-digit index
        let numbered_task = Task {
            id: 3,
            content: "Test task".to_string(),
            container: TaskContainer::Taskpad,
            created_at: now,
            status: TaskStatus::Todo,
            parent_id: None,
            child_ids: Vec::new(),
        };
        let line = format_task_line(
            &TaskIndex { path: vec![10] },
            &numbered_task,
            &display_state,
            20,
            false,
        ); // Testing with index 10
        let full_content: String = line.spans.iter().map(|span| &span.content[..]).collect();
        assert!(full_content.starts_with("10. "));
    }

    #[test]
    fn test_create_task_lines() {
        let now = Utc::now();
        let display_state = DisplayContainerState::new();
        let tasks = vec![
            Task {
                id: 1,
                content: "Task 1".to_string(),
                container: TaskContainer::Taskpad,
                created_at: now,
                status: TaskStatus::Todo,
                parent_id: None,
                child_ids: Vec::new(),
            },
            Task {
                id: 2,
                content: "Task 2".to_string(),
                container: TaskContainer::Taskpad,
                created_at: now,
                status: TaskStatus::Todo,
                parent_id: None,
                child_ids: Vec::new(),
            },
            Task {
                id: 3,
                content: "Task 3".to_string(),
                container: TaskContainer::Archived,
                created_at: now,
                status: TaskStatus::Todo,
                parent_id: None,
                child_ids: Vec::new(),
            },
        ];

        let lines = create_task_lines(
            &tasks,
            &display_state,
            20,
            None,
        );
        assert_eq!(
            lines.len(),
            3,
            "Should include 'Create new task' and two tasks"
        );
        assert!(lines[0].spans[0].content.contains("<Create new task>"));

        // Check task lines by combining their spans
        let task1_content: String = lines[1].spans.iter().map(|span| &span.content[..]).collect();
        let task2_content: String = lines[2].spans.iter().map(|span| &span.content[..]).collect();
        assert!(task1_content.contains("1. Task 1"));
        assert!(task2_content.contains("2. Task 2"));
    }

    #[test]
    fn test_calculate_cursor_position() {
        // Test cursor at start
        let (x, y) = calculate_cursor_position(0, 10);
        assert_eq!((x, y), (0, 0));

        // Test cursor on first line
        let (x, y) = calculate_cursor_position(5, 10);
        assert_eq!((x, y), (5, 0));

        // Test cursor wrapping to second line
        let (x, y) = calculate_cursor_position(12, 10);
        assert_eq!((x, y), (2, 1));

        // Test cursor at exact line end
        let (x, y) = calculate_cursor_position(10, 10);
        assert_eq!((x, y), (0, 1));
    }

    #[test]
    fn test_input_matches_focused_task() {
        let mut state = DisplayContainerState::new();
        let tasks = vec![
            Task {
                id: 1,
                content: "Task 1".to_string(),
                container: TaskContainer::Taskpad,
                created_at: Utc::now(),
                status: TaskStatus::Todo,
                parent_id: None,
                child_ids: Vec::new(),
            },
            Task {
                id: 2,
                content: "Task 2".to_string(),
                container: TaskContainer::Taskpad,
                created_at: Utc::now(),
                status: TaskStatus::Todo,
                parent_id: None,
                child_ids: Vec::new(),
            },
            Task {
                id: 3,
                content: "Task 3".to_string(),
                container: TaskContainer::Archived,
                created_at: Utc::now(),
                status: TaskStatus::Todo,
                parent_id: None,
                child_ids: Vec::new(),
            },
        ];

        // Initially at "Create new task", input should be empty
        state.update_display_order(&tasks);
        state.focused_index = Some(0);
        state.update_input_for_focus(&tasks);
        assert_eq!(state.input_value(), "");

        // Focus on first task, input should match task content
        state.focused_index = Some(1);
        state.update_input_for_focus(&tasks);
        assert_eq!(state.input_value(), "Task 1");

        // Focus on second task, input should update
        state.focused_index = Some(2);
        state.update_input_for_focus(&tasks);
        assert_eq!(state.input_value(), "Task 2");
    }

    #[test]
    fn test_input_updates_when_display_changes() {
        let mut state = DisplayContainerState::new();
        let mut tasks = vec![
            Task {
                id: 1,
                content: "Task 1".to_string(),
                container: TaskContainer::Taskpad,
                created_at: Utc::now(),
                status: TaskStatus::Todo,
                parent_id: None,
                child_ids: Vec::new(),
            },
            Task {
                id: 2,
                content: "Task 2".to_string(),
                container: TaskContainer::Taskpad,
                created_at: Utc::now(),
                status: TaskStatus::Todo,
                parent_id: None,
                child_ids: Vec::new(),
            },
            Task {
                id: 3,
                content: "Task 3".to_string(),
                container: TaskContainer::Archived,
                created_at: Utc::now(),
                status: TaskStatus::Todo,
                parent_id: None,
                child_ids: Vec::new(),
            },
        ];

        // Focus on a task
        state.update_display_order(&tasks);
        state.focused_index = Some(1);
        state.update_input_for_focus(&tasks);
        assert_eq!(state.input_value(), "Task 1");

        // Move first task to backburner, focus should stay at index 1 but show next task
        tasks[0].container = TaskContainer::Backburner;
        state.update_display_order(&tasks);
        state.update_input_for_focus(&tasks);
        assert_eq!(state.input_value(), "Task 2");
    }

    #[test]
    fn test_input_resets_when_focus_invalid() {
        let mut state = DisplayContainerState::new();
        let mut tasks = vec![
            Task {
                id: 1,
                content: "Task 1".to_string(),
                container: TaskContainer::Taskpad,
                created_at: Utc::now(),
                status: TaskStatus::Todo,
                parent_id: None,
                child_ids: Vec::new(),
            },
            Task {
                id: 2,
                content: "Task 2".to_string(),
                container: TaskContainer::Taskpad,
                created_at: Utc::now(),
                status: TaskStatus::Todo,
                parent_id: None,
                child_ids: Vec::new(),
            },
            Task {
                id: 3,
                content: "Task 3".to_string(),
                container: TaskContainer::Archived,
                created_at: Utc::now(),
                status: TaskStatus::Todo,
                parent_id: None,
                child_ids: Vec::new(),
            },
        ];

        // Focus on a task
        state.update_display_order(&tasks);
        state.focused_index = Some(1);
        state.update_input_for_focus(&tasks);
        assert_eq!(state.input_value(), "Task 1");

        // Make focus invalid by moving all tasks out of the container
        for task in tasks.iter_mut() {
            task.container = TaskContainer::Backburner;
        }
        state.update_display_order(&tasks);
        state.update_input_for_focus(&tasks);

        // Input should reset
        assert_eq!(state.input_value(), "");
    }
}
