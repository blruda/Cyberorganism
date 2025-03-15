// Keyboard input handling module
//
// This module handles keyboard input for task creation, editing, and navigation.
//
// # Focus and Input Buffer Management
//
// This module follows the unified focus and input buffer management approach
// implemented in the `DisplayContainerState.focus_task_and_update_input` method.
// For detailed documentation on this approach, see the top of the `display_container.rs` file.
//
// Key guidelines for this module:
//
// 1. Always use `focus_task_and_update_input` when changing focus
// 2. Update the GUI's input text after calling the method
// 3. Maintain focus on tasks after editing them
// 4. Do not reset focus after executing commands that explicitly change focus
// 5. IMPORTANT: Never use any form of `request_repaint()` as it causes crashes
//    and freezes in the application
//
// Following these guidelines ensures consistent behavior across all keyboard operations
// and proper synchronization between DisplayContainerState and GuiApp.

//! GUI input handling implementation.
//! 
//! This module handles keyboard and mouse input for the GUI implementation.
//! It provides functionality similar to the TUI keyhandler but adapted for egui.

use eframe::egui;
use crate::App;
use crate::commands::{Command, parse_command, execute_command, execute_create_command, execute_add_subtask};

/// Handles keyboard shortcuts and input events
pub struct KeyHandler {
    /// Whether shift key is currently pressed
    shift_pressed: bool,
    /// Whether control key is currently pressed
    ctrl_pressed: bool,
}

impl KeyHandler {
    /// Create a new key handler
    pub fn new() -> Self {
        Self {
            shift_pressed: false,
            ctrl_pressed: false,
        }
    }
    
    /// Update modifier key states based on current input
    pub fn update_modifiers(&mut self, ctx: &egui::Context) {
        // Get the current state of modifier keys in a single call to avoid inconsistencies
        ctx.input(|i| {
            self.shift_pressed = i.modifiers.shift;
            self.ctrl_pressed = i.modifiers.ctrl;
        });
    }
    
    /// Handle keyboard input for task creation or editing
    pub fn handle_input(&mut self, app: &mut App, ctx: &egui::Context, input_text: &mut String) -> bool {
        let mut handled = false;
        
        // Check for key presses
        ctx.input(|i| {
            // Handle Enter key with modifiers
            if i.key_pressed(egui::Key::Enter) {
                if self.ctrl_pressed {
                    // Toggle task status (Ctrl+Enter)
                    if let Some(index) = app.display_container_state.focused_index {
                        if index > 0 && (index - 1) < app.display_container_state.display_to_id.len() {
                            let task_id = app.display_container_state.display_to_id[index - 1];
                            
                            // First edit the task with current input text if not empty
                            if !input_text.is_empty() {
                                execute_command(app, Some(Command::Edit(task_id, input_text.clone())));
                            }
                            
                            // Find the nearest task at the same level before completing the task
                            let nearest_task_id = app.display_container_state.find_nearest_task_at_same_level(&app.tasks, task_id);
                            
                            // Complete the task by ID
                            execute_command(app, Some(Command::CompleteById(task_id)));
                            
                            // After completion, focus on the nearest task at the same level if available
                            // Otherwise, focus on the input line
                            app.display_container_state.focus_task_and_update_input(nearest_task_id, &app.tasks);
                            
                            // Update the input_text to match the display container's input value
                            *input_text = app.display_container_state.input_value().to_string();
                        }
                    }
                } else if self.shift_pressed {
                    // Create subtask (Shift+Enter)
                    if let Some(index) = app.display_container_state.focused_index {
                        if index == 0 {
                            // Create a new top-level task when on the input line
                            if !input_text.is_empty() {
                                let new_task_id = execute_create_command(app, input_text);
                                
                                // Focus on the new task using the unified method
                                app.display_container_state.focus_task_and_update_input(Some(new_task_id), &app.tasks);
                                // Update the input_text to match the display container's input value
                                *input_text = app.display_container_state.input_value().to_string();
                            }
                        } else if (index - 1) < app.display_container_state.display_to_id.len() {
                            // Create a subtask under the selected task
                            let parent_id = app.display_container_state.display_to_id[index - 1];
                            
                            // Store the original focus for returning later
                            app.display_container_state.original_focus = Some(index);
                            
                            // Create an empty subtask
                            let subtask_id = execute_add_subtask(app, &parent_id.to_string(), "");
                            
                            // Make sure the parent task is expanded
                            if !app.display_container_state.is_task_expanded(parent_id) {
                                app.display_container_state.toggle_task_expansion(parent_id, &app.tasks);
                            }
                            
                            // Update focus to the new subtask
                            if let Some(new_subtask_id) = subtask_id {
                                // Use the unified method to focus and update input
                                if app.display_container_state.focus_task_and_update_input(Some(new_subtask_id), &app.tasks) {
                                    // Update the input_text to match the display container's input value
                                    *input_text = app.display_container_state.input_value().to_string();
                                    app.log_activity(format!("Created subtask under: {}", app.tasks.iter().find(|t| t.id == parent_id).map_or("Unknown", |t| &t.content)));
                                }
                            }
                        }
                    }
                } else {
                    // Regular Enter - execute command, edit task, or create task
                    if !input_text.is_empty() {
                        let input = input_text.clone();
                        
                        // Check if we're focused on a task or the input line
                        match app.display_container_state.focused_index {
                            Some(0) | None => {
                                // On input line - parse and execute the command
                                let command = parse_command(input);
                                execute_command(app, Some(command));
                                
                                // Update the input_text to match the display container's input value
                                *input_text = app.display_container_state.input_value().to_string();
                            },
                            Some(idx) => {
                                // On a task - edit the task content
                                if (idx - 1) < app.display_container_state.display_to_id.len() {
                                    let task_id = app.display_container_state.display_to_id[idx - 1];
                                    execute_command(app, Some(Command::Edit(task_id, input)));
                                    
                                    // Check if we need to restore focus to the original task
                                    // This happens after editing a subtask created with Shift+Enter
                                    if let Some(original_idx) = app.display_container_state.original_focus {
                                        // Get the task ID for the original focus
                                        let original_task_id = if original_idx > 0 && (original_idx - 1) < app.display_container_state.display_to_id.len() {
                                            Some(app.display_container_state.display_to_id[original_idx - 1])
                                        } else {
                                            None
                                        };
                                        
                                        // Use the unified method to focus and update input
                                        if app.display_container_state.focus_task_and_update_input(original_task_id, &app.tasks) {
                                            // Update the input_text to match the display container's input value
                                            *input_text = app.display_container_state.input_value().to_string();
                                        }
                                        
                                        // Clear the original focus now that we've restored it
                                        app.display_container_state.original_focus = None;
                                    } else {
                                        // If no original focus to restore, maintain focus on the current task
                                        // This ensures we don't lose focus after editing a task
                                        app.display_container_state.focus_task_and_update_input(Some(task_id), &app.tasks);
                                        // Update the input_text to match the display container's input value
                                        *input_text = app.display_container_state.input_value().to_string();
                                    }
                                    
                                    // Don't clear the input field when editing a task
                                }
                            }
                        }
                        
                        // Don't request focus after processing command
                        // This was causing a crash when pressing Enter
                        // request_input_focus(ctx);
                    }
                }
                handled = true;
            }
            
            // Handle navigation keys - only if Ctrl is not pressed
            if !self.ctrl_pressed {
                if i.key_pressed(egui::Key::ArrowUp) {
                    if let Some(index) = app.display_container_state.focused_index {
                        if index > 0 {
                            // Calculate the task ID to focus on
                            let task_id = if index == 1 {
                                // Moving from index 1 to 0 (input line)
                                None
                            } else if (index - 1) < app.display_container_state.display_to_id.len() {
                                // Moving to a task (index - 2 because we're going up and display_to_id is 0-indexed)
                                Some(app.display_container_state.display_to_id[index - 2])
                            } else {
                                None
                            };
                            
                            // Use the unified method to focus and update input
                            if app.display_container_state.focus_task_and_update_input(task_id, &app.tasks) {
                                // Update the input_text to match the display container's input value
                                *input_text = app.display_container_state.input_value().to_string();
                                handled = true;
                            }
                        }
                    }
                }
                
                if i.key_pressed(egui::Key::ArrowDown) {
                    if let Some(index) = app.display_container_state.focused_index {
                        if index < app.display_container_state.display_to_id.len() {
                            // Calculate the task ID to focus on
                            let task_id = if index == 0 {
                                // Moving from input line (index 0) to first task
                                if !app.display_container_state.display_to_id.is_empty() {
                                    Some(app.display_container_state.display_to_id[0])
                                } else {
                                    None
                                }
                            } else if (index - 1) < app.display_container_state.display_to_id.len() - 1 {
                                // Moving to next task (index - 1 + 1 because display_to_id is 0-indexed)
                                Some(app.display_container_state.display_to_id[index])
                            } else {
                                None
                            };
                            
                            // Use the unified method to focus and update input
                            if app.display_container_state.focus_task_and_update_input(task_id, &app.tasks) {
                                // Update the input_text to match the display container's input value
                                *input_text = app.display_container_state.input_value().to_string();
                                handled = true;
                            }
                        }
                    }
                }
            }
            
            // Handle task expansion/collapse with Ctrl+Up/Down
            if self.ctrl_pressed {
                // Prevent regular arrow key handling when Ctrl is pressed
                if i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::ArrowDown) {
                    if let Some(index) = app.display_container_state.focused_index {
                        // Remember that index 0 is the "Create new task" option, so we need to subtract 1
                        if index > 0 && (index - 1) < app.display_container_state.display_to_id.len() {
                            let task_id = app.display_container_state.display_to_id[index - 1];
                            if let Some(task) = app.tasks.iter().find(|t| t.id == task_id) {
                                if !task.child_ids.is_empty() {
                                    app.display_container_state.toggle_task_expansion(task_id, &app.tasks);
                                    if let Some(display_idx) = app.display_container_state.get_display_index(task_id) {
                                        app.log_activity(format!("Toggled expansion of task {}", display_idx + 1)); // +1 because display indices are 1-based
                                    }
                                    handled = true;
                                }
                            }
                        }
                    }
                }
            }
        });
        
        // Update display order after any interaction
        app.display_container_state.update_display_order(&app.tasks);
        
        handled
    }
}
