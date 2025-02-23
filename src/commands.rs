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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup_test_app() -> App {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let tasks_file = temp_dir.path().join("tasks.json").to_str().unwrap().to_string();
        
        let mut app = App::default();
        app.tasks_file = tasks_file;
        app.tasks.push(Task::new(1, "Buy groceries".to_string()));
        app.tasks.push(Task::new(2, "Call dentist".to_string()));
        app.tasks.push(Task::new(3, "Write report".to_string()));
        app.next_id = 4;
        app
    }

    #[test]
    fn test_parse_command() {
        // Test create command (default)
        let cmd = parse_command("Buy milk".to_string());
        assert!(matches!(cmd, Command::Create(content) if content == "Buy milk"));

        // Test complete command
        let cmd = parse_command("complete Test task".to_string());
        assert!(matches!(cmd, Command::Complete(content) if content == "Test task"));

        // Test delete command
        let cmd = parse_command("delete Test task".to_string());
        assert!(matches!(cmd, Command::Delete(content) if content == "Test task"));

        // Test with trailing spaces in task content
        let cmd = parse_command("complete Test task  ".to_string());
        assert!(matches!(cmd, Command::Complete(content) if content == "Test task  "));
    }

    #[test]
    fn test_find_task_by_partial_content() {
        let app = setup_test_app();
        
        // Should match exact content
        let index = find_task(&app, "Buy groceries");
        assert!(index.is_some());
        assert_eq!(app.tasks[index.unwrap()].content, "Buy groceries");

        // Should not match partial content
        assert!(find_task(&app, "groceries").is_none());
    }

    #[test]
    fn test_find_task_by_exact_content() {
        let app = setup_test_app();
        
        // Find by exact content
        let index = find_task(&app, "Buy groceries");
        assert!(index.is_some());
        assert_eq!(app.tasks[index.unwrap()].content, "Buy groceries");
    }

    #[test]
    fn test_find_task_by_display_index() {
        let mut app = setup_test_app();
        app.taskpad_state.update_display_order(&app.tasks);
        
        // Find by display index
        let index = find_task(&app, "1");
        assert!(index.is_some());
    }

    #[test]
    fn test_find_nonexistent_task() {
        let app = setup_test_app();
        assert!(find_task(&app, "nonexistent task").is_none());
    }

    #[test]
    fn test_find_deleted_task() {
        let mut app = setup_test_app();
        let initial_count = app.tasks.len();
        
        // First find and delete a task
        let index = find_task(&app, "Buy groceries").unwrap();
        app.tasks.remove(index);
        assert_eq!(app.tasks.len(), initial_count - 1);
        
        // Now try to find it again
        assert!(find_task(&app, "Buy groceries").is_none());
    }

    #[test]
    fn test_complete_task_success() {
        let mut app = setup_test_app();
        let result = complete_task(&mut app, "Buy groceries");
        assert!(matches!(result, CommandResult::TaskCompleted { content } if content == "Buy groceries"));
    }

    #[test]
    fn test_complete_already_archived_task() {
        let mut app = setup_test_app();
        
        // First complete the task
        let _ = complete_task(&mut app, "Buy groceries");
        
        // Try to complete it again
        let result = complete_task(&mut app, "Buy groceries");
        assert!(matches!(result, CommandResult::TaskAlreadyArchived(content) if content == "Buy groceries"));
    }

    #[test]
    fn test_complete_nonexistent_task() {
        let mut app = setup_test_app();
        let result = complete_task(&mut app, "nonexistent task");
        assert!(matches!(result, CommandResult::NoMatchingTask));
    }

    #[test]
    fn test_delete_task_by_content() {
        let mut app = setup_test_app();
        let initial_count = app.tasks.len();
        
        // Delete by content match
        execute_command(&mut app, Some(Command::Delete("Buy groceries".to_string())));
        assert_eq!(app.tasks.len(), initial_count - 1);
        assert!(app.tasks.iter().all(|t| t.content != "Buy groceries"));
    }

    #[test]
    fn test_delete_task_by_index() {
        let mut app = setup_test_app();
        let initial_count = app.tasks.len();
        
        // Update display order first
        app.taskpad_state.update_display_order(&app.tasks);
        
        // Delete by index
        execute_command(&mut app, Some(Command::Delete("1".to_string())));
        assert_eq!(app.tasks.len(), initial_count - 1);
    }

    #[test]
    fn test_delete_nonexistent_task() {
        let mut app = setup_test_app();
        let initial_count = app.tasks.len();
        
        // Try to delete nonexistent task
        execute_command(&mut app, Some(Command::Delete("nonexistent task".to_string())));
        assert_eq!(app.tasks.len(), initial_count);
    }

    #[test]
    fn test_delete_completed_task() {
        let mut app = setup_test_app();
        let initial_count = app.tasks.len();
        
        // First complete a task
        let _ = complete_task(&mut app, "Buy groceries");
        
        // Then delete it
        execute_command(&mut app, Some(Command::Delete("Buy groceries".to_string())));
        assert_eq!(app.tasks.len(), initial_count - 1);
        assert!(app.tasks.iter().all(|t| t.content != "Buy groceries"));
    }
}
