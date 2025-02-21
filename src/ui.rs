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

impl TaskpadState {
    /// Creates a new TaskpadState
    pub fn new() -> Self {
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

    // Calculate height needed
    let total_height = (lines.len() + 2) as u16; // +2 for borders

    // Now create final layout with calculated height
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),               // Taskpad - take remaining space
            Constraint::Length(1),            // Help message - single line
            Constraint::Length(total_height), // Input - exact height needed
        ])
        .split(frame.size());

    // Create input widget with pre-wrapped lines (no word wrapping needed)
    let input = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Rgb(57, 255, 20)));

    // Render tasks
    let tasks_text: Vec<Line> = app
        .tasks
        .iter()
        .enumerate()
        .map(|(idx, task)| {
            Line::from(vec![Span::styled(
                format!("{}. {}", idx + 1, task.content),
                Style::default().fg(Color::Rgb(57, 255, 20)),
            )])
        })
        .collect();

    let tasks = Paragraph::new(tasks_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Rgb(57, 255, 20)))
        .wrap(Wrap { trim: false }); // Enable character wrapping for tasks too
    frame.render_widget(tasks, chunks[0]);

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
        frame.render_widget(help, chunks[1]);
    }

    // Render input
    frame.render_widget(input, chunks[2]);

    // Calculate cursor position
    let cursor_x = cursor_position as u16 % available_width as u16;
    let cursor_y = cursor_position as u16 / available_width as u16;

    // Set cursor position accounting for borders
    frame.set_cursor(chunks[2].x + 1 + cursor_x, chunks[2].y + 1 + cursor_y);
}
