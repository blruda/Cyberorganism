use chrono::Utc;
use crossterm::event::KeyCode;

use crate::taskstore::{Task, TaskContainer, TaskStatus, save_tasks};
use crate::App;

pub fn handle_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char(c) => {
            app.input.push(c);
        }
        KeyCode::Backspace => {
            app.input.pop();
        }
        KeyCode::Enter => {
            create_task(app);
        }
        _ => {}
    }
}

// Create a new task from the current input
fn create_task(app: &mut App) {
    if !app.input.trim().is_empty() {
        let task = Task {
            id: app.next_id,
            content: app.input.trim().to_string(),
            created_at: Utc::now(),
            status: TaskStatus::Todo,
            container: TaskContainer::Taskpad,
        };
        app.tasks.push(task);
        app.next_id += 1;
        app.input.clear();
        
        // Save tasks after creating a new one
        if let Err(e) = save_tasks(&app.tasks) {
            eprintln!("Failed to save tasks: {e}");
        }
    }
}
