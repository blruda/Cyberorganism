#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::items_after_statements)]
#![allow(dead_code)] // Allow dead code in TUI implementation as we're transitioning to GUI

//! Rendering functions for the terminal user interface using ratatui.
//! Handles drawing the UI components and terminal setup/teardown.

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::io;

/// The primary accent color used throughout the UI
const ACCENT_COLOR: Color = Color::Rgb(57, 255, 20);

use crate::{
    App,
    display_container::{DisplayContainerState, TaskIndex},
    taskstore::{Task, TaskContainer, TaskStatus},
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

    // Define the style to use for all spans if the task is focused
    let focused_style = Style::default().add_modifier(Modifier::REVERSED);

    // Add task index
    spans.push(if is_focused {
        Span::styled(format!("{indent}{task_index}"), focused_style)
    } else {
        Span::raw(format!("{indent}{task_index}"))
    });

    // Only add period after index for top-level tasks
    if depth == 0 {
        spans.push(if is_focused {
            Span::styled(". ", focused_style)
        } else {
            Span::raw(". ")
        });
    } else {
        spans.push(if is_focused {
            Span::styled(" ", focused_style)
        } else {
            Span::raw(" ")
        });
    }

    // Add completion status indicator
    if task.status == TaskStatus::Done {
        spans.push(if is_focused {
            Span::styled("✓ ", focused_style.fg(ACCENT_COLOR))
        } else {
            Span::styled("✓ ", Style::default().fg(ACCENT_COLOR))
        });
    }

    // Add expansion indicator if task has children
    if !task.child_ids.is_empty() {
        let indicator = if display_state.is_task_expanded(task.id) {
            "▼ "
        } else {
            "▶ "
        };
        spans.push(if is_focused {
            Span::styled(indicator, focused_style)
        } else {
            Span::raw(indicator)
        });
    }

    // Calculate remaining width for task content
    let prefix_width = indent.len() + task_index.to_string().len() + 2; // index + ". "
    let status_width = if task.status == TaskStatus::Done {
        2
    } else {
        0
    }; // "✓ "
    let indicator_width = if task.child_ids.is_empty() { 0 } else { 2 }; // "▼ " or "▶ "
    let content_width =
        available_width.saturating_sub(prefix_width + status_width + indicator_width);

    // Add task content
    let content = if task.content.len() > content_width {
        format!("{}...", &task.content[..content_width.saturating_sub(3)])
    } else {
        task.content.clone()
    };

    spans.push(if is_focused {
        Span::styled(content, focused_style)
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

    // Add the "Create new task or enter commands" entry at index 0
    let create_task_style = if focused_index == Some(0) {
        Style::default().fg(Color::Black).bg(ACCENT_COLOR)
    } else {
        Style::default().fg(ACCENT_COLOR)
    };
    lines.push(Line::from(vec![Span::styled(
        "<Create new task or enter commands>",
        create_task_style,
    )]));

    // Helper function to recursively add tasks and their subtasks
    #[allow(clippy::too_many_arguments)]
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
        let task_index = TaskIndex::from_str(
            &current_index
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("."),
        )
        .unwrap();

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
        .filter(|task| task.container == display_state.active_container && task.parent_id.is_none())
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
        Span::styled("Press ".to_string(), Style::default().fg(ACCENT_COLOR)),
        Span::styled(
            "esc".to_string(),
            Style::default()
                .fg(ACCENT_COLOR)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" or ".to_string(), Style::default().fg(ACCENT_COLOR)),
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
    use crate::display_container::ActivityLog;
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
            TaskBuilder::new(2)
                .container(TaskContainer::Archived)
                .build(),
            TaskBuilder::new(3).content("Task 1.1").parent(1).build(),
        ];

        // Start with task 1 folded
        state.fold_task(1);
        state.update_display_order(&tasks);
        // println!("Initial display_to_id: {:?}", state.display_to_id);
        // println!("Initial len: {}", state.len());
        // println!("Task 1's children: {:?}", tasks[0].child_ids);
        // println!("Task 1 expanded? {}", state.is_task_expanded(1));
        assert_eq!(state.len(), 1); // Only task 1 is visible
        assert_eq!(state.get_task_id_by_path("1", &tasks), Some(1));
        assert_eq!(state.get_task_id_by_path("1.1", &tasks), None); // Not visible until parent is expanded

        // After expanding task 1, its child becomes visible
        state.toggle_task_expansion(1, &tasks);
        // println!("\nAfter expansion:");
        // println!("Task 1 expanded? {}", state.is_task_expanded(1));
        // Note: update_display_order is now called automatically by toggle_task_expansion
        // println!("display_to_id after expansion: {:?}", state.display_to_id);
        // println!("len after expansion: {}", state.len());
        assert_eq!(state.len(), 2); // Now both task 1 and task 3 are visible
        assert_eq!(state.get_task_id_by_path("1", &tasks), Some(1));
        assert_eq!(state.get_task_id_by_path("1.1", &tasks), Some(3));

        // Change container to archive
        state.active_container = TaskContainer::Archived;
        state.update_display_order(&tasks);
        // println!("\nAfter archive:");
        // println!("display_to_id in archive: {:?}", state.display_to_id);
        // println!("len in archive: {}", state.len());
        assert_eq!(state.len(), 1); // Only task 2 is visible
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
        assert_eq!(display.get_task_id_by_path("1", &tasks), Some(7)); // First top-level task
        assert_eq!(display.get_task_id_by_path("2", &tasks), Some(6)); // Second top-level task
        assert_eq!(display.get_task_id_by_path("1.1", &tasks), Some(9)); // First subtask of first task

        // Test invalid paths
        assert_eq!(display.get_task_id_by_path("3", &tasks), None); // Non-existent top-level task
        assert_eq!(display.get_task_id_by_path("1.2", &tasks), None); // Non-existent subtask
        assert_eq!(display.get_task_id_by_path("2.1", &tasks), None); // Subtask of task with no children
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

        let lines = create_task_lines(&tasks, &display_state, 20, None);
        assert_eq!(
            lines.len(),
            3,
            "Should include 'Create new task or enter commands' and two tasks"
        );
        assert!(
            lines[0].spans[0]
                .content
                .contains("<Create new task or enter commands>")
        );

        // Check task lines by combining their spans
        let task1_content: String = lines[1]
            .spans
            .iter()
            .map(|span| &span.content[..])
            .collect();
        let task2_content: String = lines[2]
            .spans
            .iter()
            .map(|span| &span.content[..])
            .collect();
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

        // Initially at "Create new task or enter commands", input should be empty
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

    #[test]
    fn test_folded_tasks_not_in_display_list() {
        // This test ensures that when a task is folded, its subtasks are properly
        // removed from the display_to_id list and can't be navigated to.
        // This test specifically targets the bug where folded subtasks were still
        // appearing in the display_to_id list.

        // Create a task hierarchy: Task 1 -> Task 2 -> Task 3
        let tasks = vec![
            TaskBuilder::new(1)
                .content("Task 1")
                .children(vec![2])
                .build(),
            TaskBuilder::new(2)
                .content("Task 2")
                .parent(1)
                .children(vec![3])
                .build(),
            TaskBuilder::new(3).content("Task 3").parent(2).build(),
        ];

        let mut state = DisplayContainerState::default();
        state.active_container = TaskContainer::Taskpad;

        // By default, all tasks are expanded, so update_display_order should show all tasks
        state.update_display_order(&tasks);
        assert_eq!(state.display_to_id.len(), 3);
        assert_eq!(state.display_to_id[0], 1);
        assert_eq!(state.display_to_id[1], 2);
        assert_eq!(state.display_to_id[2], 3);

        // Fold Task 1, which should remove both Task 2 and Task 3 from the display list
        state.toggle_task_expansion(1, &tasks);
        assert_eq!(state.display_to_id.len(), 1);
        assert_eq!(state.display_to_id[0], 1);

        // Make sure we can't navigate to the folded tasks
        // Set focus to index 0 ("Create new task or enter commands" entry)
        state.focused_index = Some(0);

        // Try to navigate to the next task (Task 1)
        state.focus_next();
        assert_eq!(state.focused_index, Some(1)); // Should move to Task 1 (index 1)

        // Expand Task 1 again, which should make Task 2 visible again
        state.toggle_task_expansion(1, &tasks);
        assert_eq!(state.display_to_id.len(), 3);
        assert_eq!(state.display_to_id[0], 1);
        assert_eq!(state.display_to_id[1], 2);
        assert_eq!(state.display_to_id[2], 3);

        // Now we should be able to navigate to Task 2
        state.focused_index = Some(0);
        state.focus_next();
        assert_eq!(state.focused_index, Some(1)); // Should move to Task 2 (index 1)
    }
}
