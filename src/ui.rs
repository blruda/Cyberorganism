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

use crate::{
    taskstore::{Task, TaskContainer},
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
    /// Maps display positions to task IDs
    pub display_to_id: Vec<u32>,
    /// Currently focused task index (0-based)
    pub focused_index: Option<usize>,
    /// Input field for entering commands
    input: Input,
    /// Currently active container being displayed
    pub active_container: crate::taskstore::TaskContainer,
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
        }
    }

    /// Updates the display order based on the current tasks.
    /// Only includes tasks in the taskpad container (not archived).
    /// The display will show tasks as a numbered list starting from 1,
    /// with a special "Create new task" entry at index 0.
    pub fn update_display_order(&mut self, tasks: &[Task]) {
        use crate::debug::log_debug;
        log_debug(&format!(
            "Updating display for container: {:?}",
            self.active_container
        ));
        log_debug(&format!(
            "Total tasks: {}, Task containers: {:?}",
            tasks.len(),
            tasks
                .iter()
                .map(|t| (&t.content, &t.container))
                .collect::<Vec<_>>()
        ));
        self.display_to_id = tasks
            .iter()
            .filter(|task| task.container == self.active_container)
            .map(|task| task.id)
            .collect();

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

    /// Gets a task ID from a 1-based display index.
    /// Returns None for index 0 (Create new task) or invalid indices.
    pub fn get_task_id(&self, display_index: usize) -> Option<u32> {
        if display_index == 0 || display_index > self.display_to_id.len() {
            None
        } else {
            self.display_to_id.get(display_index - 1).copied()
        }
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
fn format_task_line(idx: usize, task: &Task, available_width: usize, is_focused: bool) -> Line {
    let index = format!("{}. ", idx + 1);
    let index_width = index.len();
    let content_width = available_width.saturating_sub(index_width);

    let content = if task.content.len() > content_width {
        format!("{}...", &task.content[..content_width.saturating_sub(3)])
    } else {
        task.content.clone()
    };

    let text = format!("{index}{content}");
    let style = if is_focused {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Rgb(57, 255, 20))
    } else {
        Style::default().fg(Color::Rgb(57, 255, 20))
    };

    Line::from(vec![Span::styled(text, style)])
}

/// Create task lines for display
fn create_task_lines(
    tasks: &[Task],
    available_width: usize,
    focused_index: Option<usize>,
    active_container: TaskContainer,
) -> Vec<Line> {
    let mut lines = Vec::new();

    // Add the "Create new task" entry at index 0
    let create_task_style = if focused_index == Some(0) {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Rgb(57, 255, 20))
    } else {
        Style::default().fg(Color::Rgb(57, 255, 20))
    };
    lines.push(Line::from(vec![Span::styled(
        "<Create new task>",
        create_task_style,
    )]));

    // Add the actual tasks, with indices starting at 1
    lines.extend(
        tasks
            .iter()
            .filter(|task| task.container == active_container)
            .enumerate()
            .map(|(idx, task)| {
                format_task_line(
                    idx, // format_task_line already adds 1 for display
                    task,
                    available_width,
                    Some(idx + 1) == focused_index, // +1 here since focus indices include "Create new task"
                )
            }),
    );

    lines
}

/// Create the tasks widget
fn create_tasks_widget<'a>(tasks_text: Vec<Line<'a>>, container_name: &'a str) -> Paragraph<'a> {
    Paragraph::new(tasks_text)
        .block(Block::default().borders(Borders::ALL).title(container_name))
        .style(Style::default().fg(Color::Rgb(57, 255, 20)))
        .wrap(Wrap { trim: false })
}

/// Create the activity log widget
fn create_activity_log_widget(message: &str) -> Paragraph<'_> {
    Paragraph::new(Line::from(vec![Span::styled(
        message,
        Style::default().fg(Color::Rgb(57, 255, 20)),
    )]))
}

/// Create the help widget
fn create_help_widget() -> Paragraph<'static> {
    Paragraph::new(vec![Line::from(vec![
        Span::styled(
            "Press ".to_string(),
            Style::default().fg(Color::Rgb(57, 255, 20)),
        ),
        Span::styled(
            "esc".to_string(),
            Style::default()
                .fg(Color::Rgb(57, 255, 20))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " or ".to_string(),
            Style::default().fg(Color::Rgb(57, 255, 20)),
        ),
        Span::styled(
            "ctrl-c".to_string(),
            Style::default()
                .fg(Color::Rgb(57, 255, 20))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " to exit cyberorganism".to_string(),
            Style::default().fg(Color::Rgb(57, 255, 20)),
        ),
    ])])
}

/// Create the input widget
fn create_input_widget(lines: Vec<Line<'_>>) -> Paragraph<'_> {
    Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Rgb(57, 255, 20)))
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
        available_width,
        app.display_container_state.focused_index,
        app.display_container_state.active_container,
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
    use crate::taskstore::{TaskContainer, TaskStatus};
    use chrono::Utc;

    fn setup_test_tasks() -> Vec<Task> {
        vec![
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
                container: TaskContainer::Archived,
                created_at: Utc::now(),
                status: TaskStatus::Done,
                parent_id: None,
                child_ids: Vec::new(),
            },
            Task {
                id: 3,
                content: "Task 3".to_string(),
                container: TaskContainer::Taskpad,
                created_at: Utc::now(),
                status: TaskStatus::Todo,
                parent_id: None,
                child_ids: Vec::new(),
            },
        ]
    }

    #[test]
    fn test_taskpad_display_order() {
        let mut state = DisplayContainerState::new();
        let now = Utc::now();
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

        state.update_display_order(&tasks);

        // Only taskpad tasks should be in display order
        assert_eq!(state.get_display_index(1), Some(1));
        assert_eq!(state.get_display_index(2), Some(2)); 
        assert_eq!(state.get_display_index(3), None); 

        // Test reverse lookup
        assert_eq!(state.get_task_id(1), Some(1));
        assert_eq!(state.get_task_id(2), Some(2)); 
        assert_eq!(state.get_task_id(3), None); 
    }

    #[test]
    fn test_taskpad_empty() {
        let mut state = DisplayContainerState::new();
        let tasks: Vec<Task> = vec![];

        state.update_display_order(&tasks);

        assert_eq!(state.get_task_id(1), None);
        assert_eq!(state.get_display_index(1), None);
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
        let line = format_task_line(0, &task, 20, false);
        assert!(line.spans[0].content.starts_with("1. Test task"));

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
        let line = format_task_line(0, &long_task, 20, false);
        assert!(
            line.spans[0].content.len() <= 20,
            "Line should be truncated to width"
        );
        assert!(
            line.spans[0].content.ends_with("..."),
            "Truncated line should end with ..."
        );

        // Test index width handling
        let numbered_task = Task {
            id: 3,
            content: "Test task".to_string(),
            container: TaskContainer::Taskpad,
            created_at: now,
            status: TaskStatus::Todo,
            parent_id: None,
            child_ids: Vec::new(),
        };
        let line = format_task_line(9, &numbered_task, 20, false); // Testing with index 10
        assert!(line.spans[0].content.starts_with("10. "));
    }

    #[test]
    fn test_create_task_lines() {
        let now = Utc::now();
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

        let lines = create_task_lines(&tasks, 20, None, TaskContainer::Taskpad);
        assert_eq!(
            lines.len(),
            3,
            "Should include 'Create new task' and two tasks"
        );
        assert!(lines[0].spans[0].content.contains("<Create new task>"));
        assert!(lines[1].spans[0].content.contains("Task 1"));
        assert!(lines[2].spans[0].content.contains("Task 2"));
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
        let tasks = setup_test_tasks();

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
        assert_eq!(state.input_value(), "Task 3");
    }

    #[test]
    fn test_input_updates_when_display_changes() {
        let mut state = DisplayContainerState::new();
        let mut tasks = setup_test_tasks();

        // Focus on first task
        state.update_display_order(&tasks);
        state.focused_index = Some(1);
        state.update_input_for_focus(&tasks);
        assert_eq!(state.input_value(), "Task 1");

        // Move first task to backburner, focus should stay at index 1 but show next task
        tasks[0].container = TaskContainer::Backburner;
        state.update_display_order(&tasks);
        state.update_input_for_focus(&tasks);
        assert_eq!(state.input_value(), "Task 3");
    }

    #[test]
    fn test_input_resets_when_focus_invalid() {
        let mut state = DisplayContainerState::new();
        let mut tasks = setup_test_tasks();

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
