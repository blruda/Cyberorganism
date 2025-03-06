//! Input handling for cyberorganism
//!
//! This module provides input handling functionality, including:
//! - Regular keyboard event handling via crossterm
//! - Special key combination detection via `device_query`

use device_query::{DeviceQuery, DeviceState, Keycode};
use std::collections::HashSet;
use std::time::Instant;

use crossterm::event::{Event, KeyCode};
use tui_input::backend::crossterm::EventHandler;

use crate::App;
use crate::commands::{
    Command, execute_add_subtask, execute_command, execute_create_command, parse_command,
};
use crate::debug::log_debug;

/// Represents a key combination detected by `device_query`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyCombination {
    /// Ctrl+Up key combination
    CtrlUp,
    /// Ctrl+Down key combination
    CtrlDown,
    /// Ctrl+Enter key combination
    CtrlEnter,
    /// Shift+Enter key combination
    ShiftEnter,
    /// No special combination detected
    None,
}

/// Tracks the state of special key combinations
pub struct KeyCombinationTracker {
    device_state: DeviceState,
    last_check: Instant,
    debounce_ms: u64,
    last_combination: KeyCombination,
    /// Tracks whether the Shift key is currently pressed
    pub shift_pressed: bool,
}

impl KeyCombinationTracker {
    /// Creates a new key combination tracker
    pub fn new(debounce_ms: u64) -> Self {
        Self {
            device_state: DeviceState::new(),
            last_check: Instant::now(),
            debounce_ms,
            last_combination: KeyCombination::None,
            shift_pressed: false,
        }
    }

    /// Checks for special key combinations
    /// Returns the detected combination if any
    pub fn check_combinations(&mut self) -> KeyCombination {
        // Get the current time
        let now = Instant::now();
        #[allow(clippy::cast_possible_truncation)]
        let elapsed = now.duration_since(self.last_check).as_millis() as u64;

        // Only check for key combos if cooldown has elapsed
        if elapsed < self.debounce_ms {
            return KeyCombination::None;
        }

        // Get the current key state
        let keys: HashSet<Keycode> = self.device_state.get_keys().into_iter().collect();

        // Update shift_pressed state
        self.shift_pressed = keys.contains(&Keycode::LShift) || keys.contains(&Keycode::RShift);

        // Debug log all detected keys when keys are pressed
        if !keys.is_empty() {
            log_debug(&format!("Detected keys: {keys:?}"));
        }

        // Check for modifier keys
        let is_ctrl_pressed =
            keys.contains(&Keycode::LControl) || keys.contains(&Keycode::RControl);
        let is_shift_pressed = keys.contains(&Keycode::LShift) || keys.contains(&Keycode::RShift);

        // If no modifiers are pressed, reset state and return None
        if !is_ctrl_pressed && !is_shift_pressed {
            // Only reset if we previously had a combination
            if self.last_combination != KeyCombination::None {
                self.last_combination = KeyCombination::None;
            }
            return KeyCombination::None;
        }

        // Determine the current combination
        let current_combination = if is_ctrl_pressed && keys.contains(&Keycode::Up) {
            KeyCombination::CtrlUp
        } else if is_ctrl_pressed && keys.contains(&Keycode::Down) {
            KeyCombination::CtrlDown
        } else if is_ctrl_pressed && keys.contains(&Keycode::Enter) {
            KeyCombination::CtrlEnter
        } else if is_shift_pressed && keys.contains(&Keycode::Enter) {
            KeyCombination::ShiftEnter
        } else {
            KeyCombination::None
        };

        // If no special combination is detected, return None
        if current_combination == KeyCombination::None {
            return KeyCombination::None;
        }

        // Determine if we should trigger this combination
        let should_trigger =
            // Always trigger if it's a different combination than the last one
            current_combination != self.last_combination ||
            // Or if enough time has passed since the last trigger
            elapsed >= self.debounce_ms;

        if should_trigger {
            // Update state and return the combination
            self.last_check = now;
            self.last_combination = current_combination;
            return current_combination;
        }

        // Otherwise, return None to indicate no new combination to handle
        KeyCombination::None
    }
}

/// Handles a detected key combination and applies it to the app state
/// Main dispatcher function for handling key combinations
pub fn handle_key_combination(app: &mut App, combination: KeyCombination) -> bool {
    match combination {
        KeyCombination::CtrlUp | KeyCombination::CtrlDown => {
            handle_ctrl_navigation(app, combination)
        }
        KeyCombination::ShiftEnter => handle_shift_enter(app),
        KeyCombination::CtrlEnter => handle_ctrl_enter(app),
        KeyCombination::None => false,
    }
}

/// Handles Ctrl+Up and Ctrl+Down key combinations for task expansion/collapse
fn handle_ctrl_navigation(app: &mut App, combination: KeyCombination) -> bool {
    log_debug(&format!(
        "Handling Ctrl+{:?}",
        if matches!(combination, KeyCombination::CtrlUp) {
            "Up"
        } else {
            "Down"
        }
    ));

    if let Some(idx) = app.display_container_state.focused_index {
        if idx > 0 {
            // Skip index 0 which is the input line
            if let Some(task_id) = app
                .display_container_state
                .display_to_id
                .get(idx - 1)
                .copied()
            {
                log_debug(&format!("Toggling expansion for task ID: {task_id}"));
                app.display_container_state.toggle_task_expansion(task_id);
                return true;
            }
        }
    }
    false
}

/// Handles Shift+Enter key combination for subtask creation
fn handle_shift_enter(app: &mut App) -> bool {
    log_debug("Handling Shift+Enter for subtask creation");

    match app.display_container_state.focused_index {
        // Handle Shift+Enter on the "Create new task" input line
        Some(0) | None => handle_shift_enter_on_input_line(app),
        // Handle Shift+Enter on an existing task
        Some(idx) => handle_shift_enter_on_task(app, idx),
    }
}

/// Handles Shift+Enter when focus is on the input line
fn handle_shift_enter_on_input_line(app: &mut App) -> bool {
    let input = app.display_container_state.input_value().to_string();
    if input.is_empty() {
        return false;
    }

    // Create a new task directly (bypass parse_command)
    let new_task_id = execute_create_command(app, &input);

    // Refresh the display to show the new task
    app.display_container_state.update_display_order(&app.tasks);

    // Find the display index for the new task
    if let Some(task_display_idx) = app.display_container_state.get_display_index(new_task_id) {
        // Store the original focus (the newly created task) for returning later
        app.display_container_state.original_focus = Some(task_display_idx);

        // Create an empty subtask for the newly created task
        let subtask_id = execute_add_subtask(app, &new_task_id.to_string(), "");

        // Make sure the parent task is expanded
        app.display_container_state
            .folded_tasks
            .remove(&new_task_id);

        // Refresh the display again to show the new subtask
        app.display_container_state.update_display_order(&app.tasks);

        // Find the newly created subtask and focus on it
        if let Some(new_subtask_id) = subtask_id {
            if let Some(subtask_display_idx) = app
                .display_container_state
                .get_display_index(new_subtask_id)
            {
                // Set focus to the new subtask and update input appropriately
                app.display_container_state.focused_index = Some(subtask_display_idx);
                app.display_container_state
                    .update_input_for_focus(&app.tasks);
            }
        }

        return true;
    }

    false
}

/// Handles Shift+Enter when focus is on an existing task
fn handle_shift_enter_on_task(app: &mut App, idx: usize) -> bool {
    // Get the task ID of the focused task
    if let Some(parent_task_id) = app
        .display_container_state
        .display_to_id
        .get(idx - 1)
        .copied()
    {
        // Store the original focus for returning later
        app.display_container_state.original_focus = Some(idx);

        // Create an empty subtask using the parent task ID directly
        let subtask_id = execute_add_subtask(app, &parent_task_id.to_string(), "");

        // Make sure the parent task is expanded
        app.display_container_state
            .folded_tasks
            .remove(&parent_task_id);

        // Refresh the display to show the new subtask
        app.display_container_state.update_display_order(&app.tasks);

        // Find the newly created subtask and focus on it
        if let Some(new_subtask_id) = subtask_id {
            if let Some(subtask_display_idx) = app
                .display_container_state
                .get_display_index(new_subtask_id)
            {
                // Set focus to the new subtask and update input appropriately
                app.display_container_state.focused_index = Some(subtask_display_idx);
                app.display_container_state
                    .update_input_for_focus(&app.tasks);
            }
        }

        return true;
    }

    false
}

/// Handles Ctrl+Enter key combination for task completion
fn handle_ctrl_enter(app: &mut App) -> bool {
    log_debug("Handling Ctrl+Enter");
    let input = app.display_container_state.input_value().to_string();
    if input.is_empty() || app.display_container_state.focused_index.is_none() {
        return false;
    }

    match app.display_container_state.focused_index {
        Some(0) | None => {
            // Complete the task from input
            let _ = execute_command(app, Some(Command::Complete(input)));
        }
        Some(idx) => {
            if let Some(task_id) = app
                .display_container_state
                .display_to_id
                .get(idx - 1)
                .copied()
            {
                // Edit and complete the focused task
                let _ = execute_command(app, Some(Command::Edit(task_id, input)));
                let _ = execute_command(app, Some(Command::CompleteById(task_id)));
            }
        }
    }

    true
}

/// New implementation of input event handling with cleaner structure
/// using match statements for different stages of input processing
#[allow(clippy::needless_pass_by_value)]
pub fn handle_input_event(app: &mut App, event: Event, key_tracker: &KeyCombinationTracker) {
    match event {
        Event::Key(key_event) => {
            // Handle regular key events with match statements
            match key_event.code {
                // Navigation keys
                KeyCode::Up | KeyCode::Down | KeyCode::Esc => {
                    handle_navigation_keys(app, key_event.code);
                }

                // Enter key (without modifiers from device_query)
                KeyCode::Enter => {
                    // Skip Enter key processing if Shift is being held down
                    // This prevents crossterm from intercepting Shift+Enter before device_query can detect it
                    if key_tracker.shift_pressed {
                        return;
                    }
                    // First update the input field
                    app.display_container_state
                        .get_input_mut()
                        .handle_event(&event);

                    // Then process the command
                    handle_enter_command(app);
                }

                // All other keys - pass to the input field handler
                _ => {
                    app.display_container_state
                        .get_input_mut()
                        .handle_event(&event);
                }
            }
        }
        _ => {} // Ignore non-keyboard events
    }
}

/// Handle basic navigation keys
fn handle_navigation_keys(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Up => {
            if let Some(current) = app.display_container_state.focused_index {
                // If at index 0, wrap to the last item
                let new_index = if current == 0 {
                    app.display_container_state.display_to_id.len()
                } else {
                    current - 1
                };
                app.display_container_state.focused_index = Some(new_index);
                app.display_container_state
                    .update_input_for_focus(&app.tasks);
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
                app.display_container_state
                    .update_input_for_focus(&app.tasks);
            }
        }
        KeyCode::Esc => app.display_container_state.clear_focus(),
        _ => {} // Should never happen due to the caller's match statement
    }
}

/// Handle regular Enter key command processing
#[allow(clippy::option_if_let_else)]
fn handle_enter_command(app: &mut App) {
    let input = app.display_container_state.input_value().to_string();
    if input.is_empty() {
        return;
    }

    let commands = match app.display_container_state.focused_index {
        Some(0) | None => vec![parse_command(input)],
        Some(idx) => {
            if let Some(task_id) = app
                .display_container_state
                .display_to_id
                .get(idx - 1)
                .copied()
            {
                vec![Command::Edit(task_id, input)]
            } else {
                vec![]
            }
        }
    };

    // Execute commands and collect any returned task IDs
    // Execute each command but don't track the created task IDs
    for cmd in commands {
        let _ = execute_command(app, Some(cmd));
    }

    // Check if we need to restore focus to the original task
    // This happens after editing a subtask created with Shift+Enter
    if let Some(original_idx) = app.display_container_state.original_focus {
        // Restore the original focus
        app.display_container_state.focused_index = Some(original_idx);
        // Update the input field to show the original task's content
        app.display_container_state
            .update_input_for_focus(&app.tasks);
        // Clear the original focus now that we've restored it
        app.display_container_state.original_focus = None;

        log_debug("Restored focus to parent task after subtask creation");
    }
}
