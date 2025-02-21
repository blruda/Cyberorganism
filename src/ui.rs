#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::wildcard_imports)]

//! Terminal user interface implementation using ratatui. Manages terminal setup,
//! teardown, and rendering of the task management interface.

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};
use std::io;

use crate::{App, Task};

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

/// Maintains the display state of tasks in the taskpad
pub struct TaskpadState {
    /// Maps 1-based display indices to task IDs
    display_to_id: Vec<u32>,
}

impl Default for TaskpadState {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskpadState {
    /// Creates a new `TaskpadState`
    pub const fn new() -> Self {
        Self {
            display_to_id: Vec::new(),
        }
    }

    /// Updates the display order based on the current tasks
    pub fn update_display_order(&mut self, tasks: &[Task]) {
        self.display_to_id = tasks.iter().map(|task| task.id).collect();
    }

    /// Gets a task ID from a 1-based display index
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

/// Renders the current application state to the terminal.
///
/// ### Arguments
/// * `frame` - Current frame to render to
/// * `app` - Application state to render
///
/// ### Layout
/// * Top section: Input area for new tasks
/// * Bottom section: List of existing tasks
pub fn draw(frame: &mut Frame, app: &App) {
    // Create initial layout to get available width
    let temp_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Taskpad
            Constraint::Length(3), // Temporary input height to get width (1 line + borders)
        ])
        .split(frame.size());

    // Get available width inside borders
    let available_width = temp_chunks[1].width.saturating_sub(2) as usize;

    // Split text into fixed-width lines (character wrapping)
    let input_value = app.input.value();
    let cursor_position = app.input.cursor();

    // Calculate how many lines we need based on both text and cursor position
    let needed_lines = ((cursor_position.max(input_value.len())) / available_width + 1).max(1);

    // Split text into lines, padding with empty lines if needed for cursor
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

    // Calculate height needed for input
    let total_height = (lines.len() + 2) as u16; // +2 for borders

    // Create constraints vector dynamically based on help message visibility
    let mut constraints = vec![
        Constraint::Min(1),    // Taskpad - take remaining space
        Constraint::Length(1), // Activity log - single line
    ];

    // Only add help message constraint if it's visible
    if app.show_help {
        constraints.push(Constraint::Length(1)); // Help message - single line
    }

    // Add input constraint
    constraints.push(Constraint::Length(total_height)); // Input - exact height needed

    // Create final layout with calculated height
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(frame.size());

    // Render tasks
    let tasks_area = chunks[0];
    let available_width = tasks_area.width.saturating_sub(2) as usize; // Subtract 2 for borders

    let tasks_text: Vec<Line> = app
        .tasks
        .iter()
        .enumerate()
        .map(|(idx, task)| {
            // Calculate space needed for index (e.g., "10. " = 4 chars)
            let index = format!("{}. ", idx + 1);
            let index_width = index.len();

            // Calculate remaining width for content
            let content_width = available_width.saturating_sub(index_width);

            // Truncate content if it exceeds available width
            let content = if task.content.len() > content_width {
                format!("{}...", &task.content[..content_width.saturating_sub(3)])
            } else {
                task.content.clone()
            };

            Line::from(vec![Span::styled(
                format!("{index}{content}"),
                Style::default().fg(Color::Rgb(57, 255, 20)),
            )])
        })
        .collect();

    let tasks = Paragraph::new(tasks_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Rgb(57, 255, 20)))
        .wrap(Wrap { trim: false }); // Disable wrapping since we handle it ourselves
    frame.render_widget(tasks, chunks[0]);

    // Render activity log if there's a message
    if let Some(message) = app.activity_log.latest_message() {
        let activity_log = Paragraph::new(Line::from(vec![Span::styled(
            message,
            Style::default().fg(Color::Rgb(57, 255, 20)),
        )]));
        frame.render_widget(activity_log, chunks[1]);
    }

    // Render help message if needed
    if app.show_help {
        let help = Paragraph::new(vec![Line::from(vec![
            Span::styled("Press ", Style::default().fg(Color::Rgb(57, 255, 20))),
            Span::styled(
                "esc",
                Style::default()
                    .fg(Color::Rgb(57, 255, 20))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" or ", Style::default().fg(Color::Rgb(57, 255, 20))),
            Span::styled(
                "ctrl-c",
                Style::default()
                    .fg(Color::Rgb(57, 255, 20))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to exit cyberorganism",
                Style::default().fg(Color::Rgb(57, 255, 20)),
            ),
        ])]);
        frame.render_widget(help, chunks[2]);
    }

    // Create input widget with pre-wrapped lines
    let input = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Rgb(57, 255, 20)));

    // Use the last chunk for input, accounting for help message visibility
    let input_chunk = if app.show_help { chunks[3] } else { chunks[2] };
    frame.render_widget(input, input_chunk);

    // Calculate cursor position
    let cursor_x = cursor_position as u16 % available_width as u16;
    let cursor_y = (cursor_position / available_width) as u16;

    // Set cursor position accounting for borders
    frame.set_cursor(input_chunk.x + 1 + cursor_x, input_chunk.y + 1 + cursor_y);
}
