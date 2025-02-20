//! Input handling and command processing for cyberorganism. Translates user
//! keyboard input into task management operations.

use chrono::Utc;
use crossterm::event::{self, KeyCode, KeyEvent, KeyModifiers};
use tui_input::backend::crossterm::EventHandler;

use crate::taskstore::{save_tasks, Task, TaskContainer, TaskStatus};
use crate::App;

/// Processes keyboard input and updates application state accordingly.
///
/// ### Arguments
/// * `app` - Mutable reference to the application state
/// * `key` - The keyboard key that was pressed
///
/// Handles the following keys:
/// * Enter - Creates a new task from current input
/// * Backspace - Removes last character from input
/// * Char - Adds character to input
/// * Esc/Ctrl-c - Exits the application
pub fn handle_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => {
            // Only create task if input isn't empty
            if !app.input.value().trim().is_empty() {
                create_task(app);
            }
        }
        _ => {
            // Let tui-input handle all other key events
            app.input.handle_event(&event::Event::Key(KeyEvent::new(key, KeyModifiers::empty())));
        }
    }
}

/// Creates a new task from the current input and adds it to the application state.
///
/// ### Arguments
/// * `app` - Mutable reference to the application state
///
/// The task is created with:
/// * A unique ID from the app's counter
/// * The current input text as content
/// * Current timestamp
/// * Default container (Taskpad) and status (Todo)
fn create_task(app: &mut App) {
    let task = Task {
        id: app.next_id,
        content: app.input.value().to_string(),
        created_at: Utc::now(),
        container: TaskContainer::Taskpad,
        status: TaskStatus::Todo,
    };

    app.tasks.push(task);
    app.next_id += 1;
    app.input.reset();
    app.show_help = false;

    // Save tasks after creating a new one
    if let Err(e) = save_tasks(&app.tasks) {
        eprintln!("Failed to save tasks: {e}");
    }
}
