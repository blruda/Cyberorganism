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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input area
            Constraint::Min(0),    // Task display area
        ])
        .split(frame.size());

    // Render input box
    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title("Input"));
    frame.render_widget(input, chunks[0]);

    // Render tasks
    let tasks_text: Vec<Line> = app
        .tasks
        .iter()
        .map(|task| Line::from(format!("• {}", task.content)))
        .collect();

    let tasks =
        Paragraph::new(tasks_text).block(Block::default().borders(Borders::ALL).title("Taskpad"));
    frame.render_widget(tasks, chunks[1]);
}
