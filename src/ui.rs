use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};
use std::io;

use crate::App;

// Setup terminal for TUI
pub fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).map_err(|err| io::Error::new(io::ErrorKind::Other, err))
}

// Restore terminal to normal state
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

// Draw the UI
pub fn draw(frame: &mut Frame, app: &App) {
    // Create a layout with three sections
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

    // Render tasks area with actual tasks
    let tasks_text: Vec<Line> = app
        .tasks
        .iter()
        .map(|task| Line::from(format!("• {}", task.content)))
        .collect();

    let tasks =
        Paragraph::new(tasks_text).block(Block::default().borders(Borders::ALL).title("Taskpad"));
    frame.render_widget(tasks, chunks[1]);
}
