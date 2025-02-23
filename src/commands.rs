//! Input handling and command processing for cyberorganism. Translates user
//! keyboard input into task management operations.

use crate::debug::log_debug;
use crate::taskstore::{find_task_by_content, find_task_by_id, save_tasks, Task, TaskContainer};
use crate::App;
use crossterm::event::{Event, KeyCode};
use tui_input::backend::crossterm::EventHandler;

/// Commands that can be executed by the user
enum Command {
    Create(String),
    Complete(String),
    Delete(String),
}

/// Parses the input string into a Command
#[allow(clippy::option_if_let_else)]
fn parse_command(input: String) -> Command {
    if let Some(task_query) = input.strip_prefix("complete ") {
        Command::Complete(task_query.to_string())
    } else if let Some(task_query) = input.strip_prefix("delete ") {
        Command::Delete(task_query.to_string())
    } else {
        Command::Create(input)
    }
}

/// Result of executing a command
enum CommandResult {
    TaskCompleted { content: String },
    TaskAlreadyArchived(String),
    NoMatchingTask,
}

/// Finds a task by display index or content match
fn find_task(app: &App, query: &str) -> Option<usize> {
    // Only treat as index if query is exactly one integer and nothing else
    let query = query.trim();
    if query.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(index) = query.parse::<usize>() {
            if let Some(task_id) = app.taskpad_state.get_task_id(index) {
                return find_task_by_id(&app.tasks, task_id);
            }
            log_debug(&format!("No task at index {index}"));
        }
    }

    // Fall back to fuzzy content match
    find_task_by_content(&app.tasks, query)
}

/// Completes a task by content match
fn complete_task(app: &mut App, query: &str) -> CommandResult {
    if let Some(index) = find_task(app, query) {
        let task = &mut app.tasks[index];
        if matches!(task.container(), TaskContainer::Archived) {
            CommandResult::TaskAlreadyArchived(task.content.clone())
        } else {
            let content = task.content.clone();
            task.complete();

            // Save tasks after completing one
            if let Err(e) = save_tasks(&app.tasks, &app.tasks_file) {
                log_debug(&format!("Failed to save tasks: {e}"));
            }

            CommandResult::TaskCompleted { content }
        }
    } else {
        CommandResult::NoMatchingTask
    }
}

/// Handles input events and executes commands
#[allow(clippy::needless_pass_by_value)]
pub fn handle_input_event(app: &mut App, event: Event) {
    if let Event::Key(key) = event {
        match key.code {
            KeyCode::Enter => {
                let input = app.input.value().trim().to_string();
                if input.is_empty() {
                    execute_command(app, None);
                } else {
                    let command = parse_command(input);
                    execute_command(app, Some(command));
                }
                app.input.reset();
            }
            _ => {
                app.input.handle_event(&event);
            }
        }
    }
}

/// Executes a command, updating the app state as needed
fn execute_command(app: &mut App, command: Option<Command>) {
    match command {
        Some(Command::Create(content)) => {
            let task = Task::new(app.next_id, content.clone());
            app.next_id += 1;
            app.tasks.push(task);
            app.log_activity(format!("Created task: {content}"));

            // Save tasks after creating a new one
            if let Err(e) = save_tasks(&app.tasks, &app.tasks_file) {
                log_debug(&format!("Failed to save tasks: {e}"));
            }
        }
        Some(Command::Complete(query)) => match complete_task(app, &query) {
            CommandResult::TaskCompleted { content } => {
                app.log_activity(format!("Completed task: {content}"));
            }
            CommandResult::TaskAlreadyArchived(content) => {
                app.log_activity(format!("Task '{content}' is already archived"));
            }
            CommandResult::NoMatchingTask => {
                app.log_activity("No matching task found".to_string());
            }
        },
        Some(Command::Delete(query)) => {
            if let Some(index) = find_task(app, &query) {
                let content = app.tasks[index].content.clone();
                app.tasks.remove(index);
                app.log_activity(format!("Deleted task: {content}"));

                // Save tasks after deleting one
                if let Err(e) = save_tasks(&app.tasks, &app.tasks_file) {
                    log_debug(&format!("Failed to save tasks: {e}"));
                }
            } else {
                app.log_activity("No matching task found".to_string());
            }
        }
        None => {
            app.log_activity("Invalid command".to_string());
        }
    }

    // Update display after any command
    app.taskpad_state.update_display_order(&app.tasks);
    app.show_help = false;
}
