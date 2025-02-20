//! Terminal user interface implementation using ratatui. Manages terminal setup,
//! teardown, and rendering of the task management interface.

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};
use std::io;

use crate::App;

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
            Constraint::Min(1),      // Taskpad
            Constraint::Length(3),   // Temporary input height to get width (1 line + borders)
        ])
        .split(frame.size());

    // Create input widget to calculate lines
    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Rgb(57, 255, 20)))
        .wrap(Wrap { trim: true });
    
    // Get available width inside borders
    let available_width = temp_chunks[1].width.saturating_sub(2);
    
    // Calculate needed lines (minimum 1) plus 2 for borders/title
    let needed_lines = input.line_count(available_width).max(1);
    let total_height = needed_lines.saturating_add(2) as u16;
    
    // Now create final layout with calculated height
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),                  // Taskpad - take remaining space
            Constraint::Length(total_height),    // Input - exact height needed
        ])
        .split(frame.size());

    // Render tasks
    let tasks_text: Vec<Line> = app
        .tasks
        .iter()
        .map(|task| Line::from(vec![Span::styled(
            format!("• {}", task.content),
            Style::default().fg(Color::Rgb(57, 255, 20))
        )]))
        .collect();

    let tasks =
        Paragraph::new(tasks_text)
        .block(Block::default().borders(Borders::ALL).title("Taskpad"))
        .style(Style::default().fg(Color::Rgb(57, 255, 20)));
    frame.render_widget(tasks, chunks[0]);

    // Render input
    frame.render_widget(input, chunks[1]);

    // Show cursor at input position
    // Add 1 to x and y to account for the block border
    frame.set_cursor(
        chunks[1].x + 1 + app.cursor_position as u16,
        chunks[1].y + 1
    );
}
