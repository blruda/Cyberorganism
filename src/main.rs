use serde::{Deserialize, Serialize};
use std::io;
use std::fs;
use std::path::Path;
use chrono::{DateTime, Utc};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::*,
};
use serde_json;

// Task struct represents a single task in our system
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Task {
    id: u32,                    // Unique identifier for the task
    content: String,            // The actual task content
    created_at: DateTime<Utc>,  // When the task was created
    container: TaskContainer,   // Current container of the task
    status: TaskStatus,         // Current status of the task
}

// TaskContainer represents where the task is located in our system
#[derive(Debug, Serialize, Deserialize, Clone)]
enum TaskContainer {
    Taskpad,
    Backburner,
    Shelved,
    Archived,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
enum TaskStatus {
    Todo,
    Doing,
    Done,
}

// App holds the state of our application
struct App {
    tasks: Vec<Task>,          // All tasks in the system
    input: String,             // Current input string
    next_id: u32,             // Counter for generating unique task IDs
}

impl App {
    fn new() -> App {
        // Try to load existing tasks, or start with empty vec if none exist
        let tasks = App::load_tasks().unwrap_or(Vec::new());
        // Find the highest task id to continue from
        let next_id = tasks
            .iter()
            .map(|task| task.id)
            .max()
            .unwrap_or(0) + 1;

        App {
            tasks,
            input: String::new(),
            next_id,
        }
    }

    // Handle keyboard input
    fn handle_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char(c) => {
                self.input.push(c);
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Enter => {
                self.create_task();
            }
            _ => {}
        }
    }

    // Create a new task from the current input
    fn create_task(&mut self) {
        if !self.input.trim().is_empty() {
            let task = Task {
                id: self.next_id,
                content: self.input.trim().to_string(),
                created_at: Utc::now(),
                status: TaskStatus::Todo,
                container: TaskContainer::Taskpad,
            };
            self.tasks.push(task);
            self.next_id += 1;
            self.input.clear();
            
            // Save tasks after creating a new one
            if let Err(e) = self.save_tasks() {
                eprintln!("Failed to save tasks: {}", e);
            }
        }
    }

    // Save tasks to JSON file
    fn save_tasks(&self) -> io::Result<()> {
        let json = serde_json::to_string_pretty(&self.tasks)?;
        fs::write("tasks.json", json)?;
        Ok(())
    }

    // Load tasks from JSON file
    fn load_tasks() -> io::Result<Vec<Task>> {
        if Path::new("tasks.json").exists() {
            let json = fs::read_to_string("tasks.json")?;
            Ok(serde_json::from_str(&json)?)
        } else {
            Ok(Vec::new())
        }
    }
}

// Setup terminal for TUI
fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).map_err(|err| io::Error::new(io::ErrorKind::Other, err))
}

// Restore terminal to normal state
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
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
fn ui(frame: &mut Frame, app: &App) {
    // Create a layout with three sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Input area
            Constraint::Min(0),     // Task display area
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

    let tasks = Paragraph::new(tasks_text)
        .block(Block::default().borders(Borders::ALL).title("Taskpad"));
    frame.render_widget(tasks, chunks[1]);
}

fn main() -> io::Result<()> {
    // Set up terminal
    let mut terminal = setup_terminal()?;

    // Create app state
    let mut app = App::new();

    // Main loop
    loop {
        // Draw UI
        terminal.draw(|f| ui(f, &app))?;

        // Handle input
        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('q') {
                break;
            }
            app.handle_input(key.code);
        }
    }

    // Restore terminal
    restore_terminal(&mut terminal)?;
    Ok(())
}
