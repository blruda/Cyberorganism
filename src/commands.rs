//! Input handling and command processing for cyberorganism. Translates user
//! keyboard input into task management operations.

use crossterm::event::{Event, KeyCode};
use device_query::{DeviceQuery, DeviceState, Keycode};
use tui_input::backend::crossterm::EventHandler;

use crate::{
    debug::log_debug,
    taskstore::{find_task_by_content, find_task_by_id, save_tasks, Task, TaskContainer},
    App,
};

/// Commands that can be executed by the user
enum Command {
    Create(String),
    Complete(String),
    CompleteById(u32),
    Delete(String),
    MoveToTaskpad(String),
    MoveToBackburner(String),
    MoveToShelved(String),
    Edit(u32, String), // (task_id, new_content)
    Focus(String),     // Focus on a task by index or content
}

/// Parses the input string into a Command
#[allow(clippy::option_if_let_else)]
fn parse_command(input: String) -> Command {
    if let Some(task_query) = input.strip_prefix("complete ") {
        Command::Complete(task_query.to_string())
    } else if let Some(task_query) = input.strip_prefix("delete ") {
        Command::Delete(task_query.to_string())
    } else if let Some(task_query) = input.strip_prefix("move to taskpad ") {
        Command::MoveToTaskpad(task_query.to_string())
    } else if let Some(task_query) = input.strip_prefix("move to backburner ") {
        Command::MoveToBackburner(task_query.to_string())
    } else if let Some(task_query) = input.strip_prefix("move to shelved ") {
        Command::MoveToShelved(task_query.to_string())
    } else if let Some(task_query) = input.strip_prefix("focus ") {
        Command::Focus(task_query.to_string())
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
            if let Some(task_id) = app.display_container_state.get_task_id(index) {
                return find_task_by_id(&app.tasks, task_id);
            }
            log_debug(&format!("No task at index {index}"));
        }
    }

    // Fall back to fuzzy content match
    find_task_by_content(&app.tasks, query, &app.display_container_state.active_container)
}

/// Completes a task by content match or task ID
fn complete_task(app: &mut App, query: &str, task_id: Option<u32>) -> CommandResult {
    // If task_id is provided, complete that task directly
    if let Some(id) = task_id {
        if let Some(task) = app.tasks.iter_mut().find(|t| t.id == id) {
            if matches!(task.container(), TaskContainer::Archived) {
                return CommandResult::TaskAlreadyArchived(task.content.clone());
            }
            let content = task.content.clone();
            task.complete();
            app.display_container_state.update_display_order(&app.tasks);
            return CommandResult::TaskCompleted { content };
        }
        return CommandResult::NoMatchingTask;
    }

    // Otherwise, search for task by content
    if let Some(idx) = find_task(app, query) {
        let task = &mut app.tasks[idx];
        if matches!(task.container(), TaskContainer::Archived) {
            return CommandResult::TaskAlreadyArchived(task.content.clone());
        }
        let content = task.content.clone();
        task.complete();
        app.display_container_state.update_display_order(&app.tasks);
        CommandResult::TaskCompleted { content }
    } else {
        CommandResult::NoMatchingTask
    }
}

/// Execute a create command
fn execute_create_command(app: &mut App, content: &str) {
    let task = Task::new(app.next_id, content.to_string());
    app.next_id += 1;
    app.tasks.push(task);
    app.log_activity("Task added".to_string());
    if let Err(e) = save_tasks(&app.tasks, &app.tasks_file) {
        log_debug(&format!("Failed to save tasks: {e}"));
    }
}

/// Execute a complete command
fn execute_complete_command(app: &mut App, query: &str) {
    match complete_task(app, query, None) {
        CommandResult::TaskCompleted { content } => {
            app.log_activity(format!("Completed task: {content}"));
        }
        CommandResult::TaskAlreadyArchived(content) => {
            app.log_activity(format!("Task '{content}' is already archived"));
        }
        CommandResult::NoMatchingTask => {
            app.log_activity("No matching task found".to_string());
        }
    }
}

/// Execute a complete by ID command
fn execute_complete_by_id_command(app: &mut App, task_id: u32) {
    match complete_task(app, "", Some(task_id)) {
        CommandResult::TaskCompleted { content } => {
            app.log_activity(format!("Completed task: {content}"));
        }
        CommandResult::TaskAlreadyArchived(content) => {
            app.log_activity(format!("Task '{content}' is already archived"));
        }
        CommandResult::NoMatchingTask => {
            app.log_activity("No matching task found".to_string());
        }
    }
}

/// Execute a delete command
fn execute_delete_command(app: &mut App, query: &str) {
    if let Some(index) = find_task(app, query) {
        let content = app.tasks[index].content.clone();
        app.tasks.remove(index);
        app.log_activity(format!("Deleted task: {content}"));
        if let Err(e) = save_tasks(&app.tasks, &app.tasks_file) {
            log_debug(&format!("Failed to save tasks: {e}"));
        }
    } else {
        app.log_activity("No matching task found".to_string());
    }
}

/// Execute a move command
fn execute_move_command(app: &mut App, query: &str, target_container: TaskContainer) {
    if let Some(index) = find_task(app, query) {
        let task = &mut app.tasks[index];
        if task.container == target_container {
            app.log_activity(format!(
                "Task already in {}",
                target_container.display_name()
            ));
        } else {
            let content = task.content.clone();
            let container_name = target_container.display_name();
            task.container = target_container;

            // Save tasks after moving one
            if let Err(e) = save_tasks(&app.tasks, &app.tasks_file) {
                log_debug(&format!("Failed to save tasks: {e}"));
            }

            app.log_activity(format!("Moved task to {container_name}: {content}"));
        }
    } else {
        app.log_activity("No matching task found".to_string());
    }
}

/// Execute a move to taskpad command
fn execute_move_to_taskpad_command(app: &mut App, query: &str) {
    execute_move_command(app, query, TaskContainer::Taskpad);
}

/// Execute a move to backburner command
fn execute_move_to_backburner_command(app: &mut App, query: &str) {
    execute_move_command(app, query, TaskContainer::Backburner);
}

/// Execute a move to shelved command
fn execute_move_to_shelved_command(app: &mut App, query: &str) {
    execute_move_command(app, query, TaskContainer::Shelved);
}

/// Result of focusing on a task
enum FocusResult {
    Focused { content: String },
    NoMatchingTask,
}

/// Focuses on a task by content match or display index
fn focus_task(app: &mut App, query: &str) -> FocusResult {
    if let Some(index) = find_task(app, query) {
        let task = &app.tasks[index];
        let content = task.content.clone();

        // Find the display index for this task
        if let Some(display_idx) = app.display_container_state.get_display_index(task.id) {
            app.display_container_state.focused_index = Some(display_idx);
            // Also update input buffer with task content
            app.display_container_state.set_input(&content);
            FocusResult::Focused { content }
        } else {
            FocusResult::NoMatchingTask
        }
    } else {
        FocusResult::NoMatchingTask
    }
}

/// Executes the focus command
fn execute_focus_command(app: &mut App, query: &str) {
    match focus_task(app, query) {
        FocusResult::Focused { content } => {
            app.log_activity(format!("Focused on task: {content}"));
        }
        FocusResult::NoMatchingTask => {
            app.log_activity("No matching task found".to_string());
        }
    }
}

/// Executes a command, updating the app state as needed
fn execute_command(app: &mut App, command: Option<Command>) {
    match command {
        Some(Command::Create(content)) => execute_create_command(app, &content),
        Some(Command::Complete(query)) => execute_complete_command(app, &query),
        Some(Command::CompleteById(task_id)) => execute_complete_by_id_command(app, task_id),
        Some(Command::Delete(query)) => execute_delete_command(app, &query),
        Some(Command::MoveToTaskpad(query)) => execute_move_to_taskpad_command(app, &query),
        Some(Command::MoveToBackburner(query)) => execute_move_to_backburner_command(app, &query),
        Some(Command::MoveToShelved(query)) => execute_move_to_shelved_command(app, &query),
        Some(Command::Focus(query)) => execute_focus_command(app, &query),
        Some(Command::Edit(task_id, content)) => {
            if let Some(task) = app.tasks.iter_mut().find(|t| t.id == task_id) {
                task.update_content(content);
                app.log_activity("Task updated".to_string());
                if let Err(e) = save_tasks(&app.tasks, &app.tasks_file) {
                    log_debug(&format!("Failed to save tasks: {e}"));
                }
            }
        }
        None => {
            app.log_activity("Invalid command".to_string());
        }
    }

    // Update display after any command
    app.display_container_state.update_display_order(&app.tasks);
    app.show_help = false;
}

fn is_ctrl_enter_pressed() -> bool {
    let device_state = DeviceState::new();
    let keys: Vec<Keycode> = device_state.get_keys();
    keys.contains(&Keycode::LControl) && keys.contains(&Keycode::Enter)
}

/// Handle keyboard input events
#[allow(clippy::needless_pass_by_value)] // Event is small and Copy
#[allow(clippy::single_match)] // Match is clearer and we might add more event types
pub fn handle_input_event(app: &mut App, event: Event) {
    match event {
        Event::Key(key_event) => match key_event.code {
            KeyCode::Up => {
                if let Some(current) = app.display_container_state.focused_index {
                    // If at index 0, wrap to the last item
                    let new_index = if current == 0 {
                        app.display_container_state.display_to_id.len()
                    } else {
                        current - 1
                    };
                    app.display_container_state.focused_index = Some(new_index);
                    app.display_container_state.update_input_for_focus(&app.tasks);
                }
            }
            KeyCode::Down => {
                if let Some(current) = app.display_container_state.focused_index {
                    // If at last index, wrap to 0
                    let new_index = if current >= app.display_container_state.display_to_id.len() {
                        0
                    } else {
                        current + 1
                    };
                    app.display_container_state.focused_index = Some(new_index);
                    app.display_container_state.update_input_for_focus(&app.tasks);
                }
            }
            KeyCode::Esc => app.display_container_state.clear_focus(),
            _ => {
                // Handle input field updates
                app.display_container_state.get_input_mut().handle_event(&event);

                // Check for Enter to submit
                if key_event.code == KeyCode::Enter {
                    let input = app.display_container_state.input_value().to_string();
                    if !input.is_empty() {
                        let commands = if is_ctrl_enter_pressed()
                            && app.display_container_state.focused_index.is_some()
                        {
                            match app.display_container_state.focused_index {
                                Some(0) | None => vec![Command::Complete(input)],
                                Some(idx) => app
                                    .display_container_state
                                    .display_to_id
                                    .get(idx - 1)
                                    .copied()
                                    .map_or_else(Vec::new, |task_id| {
                                        vec![
                                            Command::Edit(task_id, input),
                                            Command::CompleteById(task_id),
                                        ]
                                    }),
                            }
                        } else {
                            match app.display_container_state.focused_index {
                                Some(0) | None => vec![parse_command(input)],
                                Some(idx) => app
                                    .display_container_state
                                    .display_to_id
                                    .get(idx - 1)
                                    .copied()
                                    .map_or_else(Vec::new, |task_id| {
                                        vec![Command::Edit(task_id, input)]
                                    }),
                            }
                        };

                        for cmd in commands {
                            match cmd {
                                Command::Edit(_, _) => {
                                    // For edits, keep the input buffer and focus
                                    execute_command(app, Some(cmd));
                                }
                                _ => {
                                    // For other commands, execute
                                    execute_command(app, Some(cmd));
                                }
                            }
                        }
                    }
                }
            }
        },
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Test utilities and conventions:
    /// - When checking activity log messages, always use `activity_log.latest_message()`
    ///   instead of trying to access the log entries directly. The ActivityLog struct
    ///   provides this method specifically for getting the most recent message.
    /// - Container names in messages are always lowercase (e.g., "taskpad" not "Taskpad")

    fn setup_test_app() -> App {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let tasks_file = temp_dir
            .path()
            .join("tasks.json")
            .to_str()
            .unwrap()
            .to_string();

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

        // Test move to taskpad command
        let cmd = parse_command("move to taskpad Test task".to_string());
        assert!(matches!(cmd, Command::MoveToTaskpad(content) if content == "Test task"));

        // Test move to backburner command
        let cmd = parse_command("move to backburner Test task".to_string());
        assert!(matches!(cmd, Command::MoveToBackburner(content) if content == "Test task"));

        // Test move to shelved command
        let cmd = parse_command("move to shelved Test task".to_string());
        assert!(matches!(cmd, Command::MoveToShelved(content) if content == "Test task"));

        // Test focus command
        let cmd = parse_command("focus Test task".to_string());
        assert!(matches!(cmd, Command::Focus(content) if content == "Test task"));

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
        app.display_container_state.update_display_order(&app.tasks);

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
        let result = complete_task(&mut app, "Buy groceries", None);
        assert!(
            matches!(result, CommandResult::TaskCompleted { content } if content == "Buy groceries")
        );
    }

    #[test]
    fn test_complete_already_archived_task() {
        let mut app = setup_test_app();

        // First complete the task
        let _ = complete_task(&mut app, "Buy groceries", None);

        // Try to complete it again
        let result = complete_task(&mut app, "Buy groceries", None);
        assert!(
            matches!(result, CommandResult::TaskAlreadyArchived(content) if content == "Buy groceries")
        );
    }

    #[test]
    fn test_complete_nonexistent_task() {
        let mut app = setup_test_app();
        let result = complete_task(&mut app, "nonexistent task", None);
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
        app.display_container_state.update_display_order(&app.tasks);

        // Delete by index
        execute_command(&mut app, Some(Command::Delete("1".to_string())));
        assert_eq!(app.tasks.len(), initial_count - 1);
    }

    #[test]
    fn test_delete_nonexistent_task() {
        let mut app = setup_test_app();
        let initial_count = app.tasks.len();

        // Try to delete nonexistent task
        execute_command(
            &mut app,
            Some(Command::Delete("nonexistent task".to_string())),
        );
        assert_eq!(app.tasks.len(), initial_count);
    }

    #[test]
    fn test_delete_completed_task() {
        let mut app = setup_test_app();
        let initial_count = app.tasks.len();

        // First complete a task
        let _ = complete_task(&mut app, "Buy groceries", None);

        // Then delete it
        execute_command(&mut app, Some(Command::Delete("Buy groceries".to_string())));
        assert_eq!(app.tasks.len(), initial_count - 1);
        assert!(app.tasks.iter().all(|t| t.content != "Buy groceries"));
    }

    #[test]
    fn test_move_to_taskpad_success() {
        let mut app = setup_test_app();
        let task = &mut app.tasks[0];
        task.container = TaskContainer::Backburner;
        let content = task.content.clone();

        execute_move_to_taskpad_command(&mut app, &content);

        assert_eq!(app.tasks[0].container, TaskContainer::Taskpad);
        assert_eq!(
            app.activity_log.latest_message().unwrap(),
            &format!("Moved task to taskpad: {}", content)
        );
    }

    #[test]
    fn test_move_to_taskpad_already_there() {
        let mut app = setup_test_app();
        let task = &app.tasks[0];
        let content = task.content.clone();

        execute_move_to_taskpad_command(&mut app, &content);

        assert_eq!(app.tasks[0].container, TaskContainer::Taskpad);
        assert_eq!(
            app.activity_log.latest_message().unwrap(),
            "Task already in taskpad"
        );
    }

    #[test]
    fn test_move_to_backburner_success() {
        let mut app = setup_test_app();
        let task = &app.tasks[0];
        let content = task.content.clone();

        execute_move_to_backburner_command(&mut app, &content);

        assert_eq!(app.tasks[0].container, TaskContainer::Backburner);
        assert_eq!(
            app.activity_log.latest_message().unwrap(),
            &format!("Moved task to backburner: {}", content)
        );
    }

    #[test]
    fn test_move_to_backburner_already_there() {
        let mut app = setup_test_app();
        let task = &mut app.tasks[0];
        task.container = TaskContainer::Backburner;
        let content = task.content.clone();

        execute_move_to_backburner_command(&mut app, &content);

        assert_eq!(app.tasks[0].container, TaskContainer::Backburner);
        assert_eq!(
            app.activity_log.latest_message().unwrap(),
            "Task already in backburner"
        );
    }

    #[test]
    fn test_move_to_shelved_success() {
        let mut app = setup_test_app();
        let task = &app.tasks[0];
        let content = task.content.clone();

        execute_move_to_shelved_command(&mut app, &content);

        assert_eq!(app.tasks[0].container, TaskContainer::Shelved);
        assert_eq!(
            app.activity_log.latest_message().unwrap(),
            &format!("Moved task to shelved: {}", content)
        );
    }

    #[test]
    fn test_move_to_shelved_already_there() {
        let mut app = setup_test_app();
        let task = &mut app.tasks[0];
        task.container = TaskContainer::Shelved;
        let content = task.content.clone();

        execute_move_to_shelved_command(&mut app, &content);

        assert_eq!(app.tasks[0].container, TaskContainer::Shelved);
        assert_eq!(
            app.activity_log.latest_message().unwrap(),
            "Task already in shelved"
        );
    }

    #[test]
    fn test_move_nonexistent_task() {
        let mut app = setup_test_app();

        execute_move_to_taskpad_command(&mut app, "Nonexistent task");
        assert_eq!(
            app.activity_log.latest_message().unwrap(),
            "No matching task found"
        );

        execute_move_to_backburner_command(&mut app, "Nonexistent task");
        assert_eq!(
            app.activity_log.latest_message().unwrap(),
            "No matching task found"
        );

        execute_move_to_shelved_command(&mut app, "Nonexistent task");
        assert_eq!(
            app.activity_log.latest_message().unwrap(),
            "No matching task found"
        );
    }

    #[test]
    fn test_focus_task_by_content() {
        let mut app = setup_test_app();
        app.display_container_state.update_display_order(&app.tasks);
        let result = focus_task(&mut app, "Buy groceries");
        assert!(matches!(
            result,
            FocusResult::Focused { content } if content == "Buy groceries"
        ));
    }

    #[test]
    fn test_focus_task_by_index_updates_state() {
        let mut app = setup_test_app();
        app.display_container_state.update_display_order(&app.tasks);

        let result = focus_task(&mut app, "1");
        assert!(matches!(result, FocusResult::Focused { .. }));

        // Check that state is updated
        assert_eq!(app.display_container_state.focused_index, Some(1));
        assert_eq!(app.display_container_state.input_value(), "Buy groceries");
    }

    #[test]
    fn test_focus_task_updates_input() {
        let mut app = setup_test_app();
        app.display_container_state.update_display_order(&app.tasks);

        // Set initial input
        app.display_container_state.set_input("previous input");

        // Focus on first task
        let result = focus_task(&mut app, "1");
        assert!(matches!(result, FocusResult::Focused { .. }));

        // Check that input is updated
        assert_eq!(app.display_container_state.input_value(), "Buy groceries");
    }

    #[test]
    fn test_focus_nonexistent_task() {
        let mut app = setup_test_app();
        let result = focus_task(&mut app, "nonexistent task");
        assert!(matches!(result, FocusResult::NoMatchingTask));
    }
}
