mod commands;
mod taskstore;
mod ui;

use crossterm::event::{self, Event, KeyCode};
use std::io;
use taskstore::{load_tasks, Task};

// App holds the state of our application
pub struct App {
    pub tasks: Vec<Task>, // All tasks in the system
    pub input: String,    // Current input string
    pub next_id: u32,     // Counter for generating unique task IDs
}

impl App {
    fn new() -> Self {
        // Try to load existing tasks, or start with empty vec if none exist
        let tasks = load_tasks().unwrap_or_default();
        // Find the highest task id to continue from
        let next_id = tasks.iter().map(|task| task.id).max().unwrap_or(0) + 1;

        Self {
            tasks,
            input: String::new(),
            next_id,
        }
    }
}

fn main() -> io::Result<()> {
    // Set up terminal
    let mut terminal = ui::setup_terminal()?;

    // Create app state
    let mut app = App::new();

    // Main loop
    loop {
        // Draw UI
        terminal.draw(|f| ui::draw(f, &app))?;

        // Handle input
        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('q') {
                break;
            }
            commands::handle_input(&mut app, key.code);
        }
    }

    // Restore terminal
    ui::restore_terminal(&mut terminal)?;
    Ok(())
}
