//! GUI input handling implementation.
//! 
//! This module handles keyboard and mouse input for the GUI implementation.
//! It provides functionality similar to the TUI keyhandler but adapted for egui.

use eframe::egui;
use crate::App;
use crate::commands::{Command, parse_command, execute_command, execute_create_command, execute_add_subtask};

/// Request focus for the input field
pub fn request_input_focus(ctx: &egui::Context) {
    // Use the same consistent ID as in rendering.rs
    let input_id = egui::Id::new("main_input_field");
    ctx.memory_mut(|mem| mem.request_focus(input_id));
}

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
                            
                            // Then complete the task by ID
                            execute_command(app, Some(Command::CompleteById(task_id)));
                            
                            // Clear the input field
                            *input_text = String::new();
                            app.display_container_state.request_focus_next_frame = true;
                        }
                    }
                } else if self.shift_pressed {
                    // Create subtask (Shift+Enter)
                    if let Some(index) = app.display_container_state.focused_index {
                        if index == 0 {
                            // Create a new top-level task when on the input line
                            if !input_text.is_empty() {
                                let new_task_id = execute_create_command(app, input_text);
                                *input_text = String::new();
                                
                                // Update focus to the new task
                                if let Some(display_idx) = app.display_container_state.get_display_index(new_task_id) {
                                    app.display_container_state.focused_index = Some(display_idx + 1); // +1 because 0 is "Create new task"
                                }
                            }
                        } else if (index - 1) < app.display_container_state.display_to_id.len() {
                            // Create a subtask under the selected task
                            let parent_id = app.display_container_state.display_to_id[index - 1];
                            if let Some(parent_idx) = app.tasks.iter().position(|t| t.id == parent_id) {
                                let parent_content = app.tasks[parent_idx].content.clone();
                                
                                if !input_text.is_empty() {
                                    // Create subtask with the current input text
                                    let subtask_id = execute_add_subtask(app, &parent_id.to_string(), input_text);
                                    *input_text = String::new();
                                    
                                    // Make sure the parent task is expanded
                                    if !app.display_container_state.is_task_expanded(parent_id) {
                                        app.display_container_state.toggle_task_expansion(parent_id, &app.tasks);
                                    }
                                    
                                    // Update focus to the new subtask
                                    if let Some(new_subtask_id) = subtask_id {
                                        if let Some(display_idx) = app.display_container_state.get_display_index(new_subtask_id) {
                                            app.display_container_state.focused_index = Some(display_idx + 1); // +1 because 0 is "Create new task"
                                            app.log_activity(format!("Created subtask {} under: {}", display_idx + 1, parent_content));
                                        }
                                    }
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
                                
                                // Only clear the input field when on the input line
                                *input_text = String::new();
                                app.display_container_state.request_focus_next_frame = true;
                            },
                            Some(idx) => {
                                // On a task - edit the task content
                                if (idx - 1) < app.display_container_state.display_to_id.len() {
                                    let task_id = app.display_container_state.display_to_id[idx - 1];
                                    execute_command(app, Some(Command::Edit(task_id, input)));
                                    
                                    // Don't clear the input field when editing a task
                                    // But still request focus for the next frame
                                    app.display_container_state.request_focus_next_frame = true;
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
                            app.display_container_state.focused_index = Some(index - 1);
                            // Update the input field with the content of the newly focused task
                            app.display_container_state.update_input_for_focus(&app.tasks);
                            // Update the input_text to match the display container's input value
                            *input_text = app.display_container_state.input_value().to_string();
                            handled = true;
                        }
                    }
                }
                
                if i.key_pressed(egui::Key::ArrowDown) {
                    if let Some(index) = app.display_container_state.focused_index {
                        if index < app.display_container_state.display_to_id.len() {
                            app.display_container_state.focused_index = Some(index + 1);
                            // Update the input field with the content of the newly focused task
                            app.display_container_state.update_input_for_focus(&app.tasks);
                            // Update the input_text to match the display container's input value
                            *input_text = app.display_container_state.input_value().to_string();
                            handled = true;
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
        
        // Request focus for the input field after handling interactions
        ctx.request_repaint();
        
        handled
    }
}
