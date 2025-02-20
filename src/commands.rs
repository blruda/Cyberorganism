//! Input handling and command processing for cyberorganism. Translates user
//! keyboard input into task management operations.

use chrono::Utc;
use crossterm::event::KeyCode;

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
            if !app.input.trim().is_empty() {
                create_task(app);
                app.cursor_position = 0;
            }
        }
        KeyCode::Backspace => {
            if app.cursor_position > 0 {
                app.cursor_position -= 1;
                app.input.remove(app.cursor_position);
            }
        }
        KeyCode::Delete => {
            if app.cursor_position < app.input.len() {
                app.input.remove(app.cursor_position);
            }
        }
        KeyCode::Left => {
            if app.cursor_position > 0 {
                app.cursor_position -= 1;
            }
        }
        KeyCode::Right => {
            if app.cursor_position < app.input.len() {
                app.cursor_position += 1;
            }
        }
        KeyCode::Home => {
            app.cursor_position = 0;
        }
        KeyCode::End => {
            app.cursor_position = app.input.len();
        }
        KeyCode::Char(c) => {
            app.input.insert(app.cursor_position, c);
            app.cursor_position += 1;
        }
        _ => {}
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
        content: app.input.clone(),
        created_at: Utc::now(),
        container: TaskContainer::Taskpad,
        status: TaskStatus::Todo,
    };

    app.tasks.push(task);
    app.next_id += 1;
    app.input.clear();

    // Save tasks after creating a new one
    if let Err(e) = save_tasks(&app.tasks) {
        eprintln!("Failed to save tasks: {e}");
    }
}
