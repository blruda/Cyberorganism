//! Special key combination handling using `device_query`
//!
//! This module provides a separate polling mechanism for detecting
//! key combinations that crossterm may not reliably detect.

use device_query::{DeviceQuery, DeviceState, Keycode};
use std::collections::HashSet;
use std::time::Instant;

use crate::App;
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
    /// No special combination detected
    None,
}

/// Tracks the state of special key combinations
pub struct KeyCombinationTracker {
    device_state: DeviceState,
    last_check: Instant,
    debounce_ms: u64,
    last_combination: KeyCombination,
}

impl KeyCombinationTracker {
    /// Creates a new key combination tracker
    pub fn new(debounce_ms: u64) -> Self {
        Self {
            device_state: DeviceState::new(),
            last_check: Instant::now(),
            debounce_ms,
            last_combination: KeyCombination::None,
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

        // Debug log all detected keys when keys are pressed
        if !keys.is_empty() {
            log_debug(&format!("Detected keys: {keys:?}"));
        }

        // Check for Ctrl key
        let is_ctrl_pressed =
            keys.contains(&Keycode::LControl) || keys.contains(&Keycode::RControl);

        // If Ctrl is not pressed, reset state and return None
        if !is_ctrl_pressed {
            // Only reset if we previously had a combination
            if self.last_combination != KeyCombination::None {
                self.last_combination = KeyCombination::None;
            }
            return KeyCombination::None;
        }

        // Determine the current combination
        let current_combination = if keys.contains(&Keycode::Up) {
            KeyCombination::CtrlUp
        } else if keys.contains(&Keycode::Down) {
            KeyCombination::CtrlDown
        } else if keys.contains(&Keycode::Enter) {
            KeyCombination::CtrlEnter
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
pub fn handle_key_combination(app: &mut App, combination: KeyCombination) -> bool {
    match combination {
        KeyCombination::CtrlUp | KeyCombination::CtrlDown => {
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
        }
        KeyCombination::CtrlEnter => {
            log_debug("Handling Ctrl+Enter");
            let input = app.display_container_state.input_value().to_string();
            if !input.is_empty() && app.display_container_state.focused_index.is_some() {
                match app.display_container_state.focused_index {
                    Some(0) | None => {
                        // Complete the task from input
                        crate::commands::execute_command(
                            app,
                            Some(crate::commands::Command::Complete(input)),
                        );
                    }
                    Some(idx) => {
                        if let Some(task_id) = app
                            .display_container_state
                            .display_to_id
                            .get(idx - 1)
                            .copied()
                        {
                            // Edit and complete the focused task
                            crate::commands::execute_command(
                                app,
                                Some(crate::commands::Command::Edit(task_id, input)),
                            );
                            crate::commands::execute_command(
                                app,
                                Some(crate::commands::Command::CompleteById(task_id)),
                            );
                        }
                    }
                }
                return true;
            }
        }
        KeyCombination::None => {}
    }

    false
}
