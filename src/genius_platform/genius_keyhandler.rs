//! Genius Feed input handling module
//! 
//! This module handles keyboard input specifically for the Genius Feed mode.
//! It is separate from the main keyhandler.rs to avoid introducing errors
//! in the complex PKM input handling logic.

use eframe::egui;
use crate::App;
use crate::commands;
use crate::gui::genius_feed;
use crate::genius_platform::GeniusApiBridge;

/// Handles keyboard input for Genius Feed mode
pub struct GeniusKeyHandler {
    /// Whether shift key is currently pressed
    shift_pressed: bool,
    /// Whether control key is currently pressed
    ctrl_pressed: bool,
}

impl GeniusKeyHandler {
    /// Create a new Genius key handler
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
    
    /// Handle keyboard input for Genius Feed mode
    /// 
    /// This function handles keyboard input specifically for the Genius Feed mode.
    /// It implements navigation within the Genius Feed results and keyboard shortcuts
    /// for mode switching.
    pub fn handle_input(&mut self, app: &mut App, ctx: &egui::Context, input_text: &mut String) -> bool {
        let mut handled = false;
        
        // Update modifier keys
        self.update_modifiers(ctx);
        
        // Check if we should query the API based on input changes
        genius_feed::maybe_query_api(app, input_text);
        
        // Use our new handle_keyboard_navigation function for arrow key navigation
        genius_feed::handle_keyboard_navigation(&GeniusApiBridge::global(), ctx);
        
        // Check for key presses
        ctx.input(|i| {
            // Mode switching with Ctrl+Space
            if i.key_pressed(egui::Key::Space) && self.ctrl_pressed {
                // Toggle the app mode
                let previous_mode = app.app_mode;
                app.app_mode = commands::toggle_app_mode(app, app.app_mode);
                
                // If switching from Feed to PKM mode, refresh the input buffer
                if previous_mode == crate::commands::AppMode::Feed && 
                   app.app_mode == crate::commands::AppMode::Pkm {
                    // Use focus_task_and_update_input to refresh the input buffer
                    // This ensures we don't accidentally edit a focused task with the query text
                    app.display_container_state.focus_task_and_update_input(
                        app.display_container_state.focused_index.and_then(|idx| {
                            if idx > 0 && idx - 1 < app.display_container_state.display_to_id.len() {
                                Some(app.display_container_state.display_to_id[idx - 1])
                            } else {
                                None
                            }
                        }),
                        &app.tasks
                    );
                    
                    // Update the input_text to match the display container's input value
                    *input_text = app.display_container_state.input_value().to_string();
                }
                
                handled = true;
            }
            
            // Get the number of items in the feed for navigation bounds
            let item_count = GeniusApiBridge::global().all_items().len();
            
            // Handle Ctrl+Up/Down for toggling expansion
            if i.key_pressed(egui::Key::ArrowUp) && self.ctrl_pressed {
                // Ctrl+Up toggles expansion of the currently focused item
                if let Some(focused_idx) = crate::gui::genius_feed::GeniusFeedState::get_focused_index() {
                    if focused_idx < item_count {
                        crate::gui::genius_feed::GeniusFeedState::toggle_item_expansion(focused_idx);
                        handled = true;
                    }
                }
            }
            
            if i.key_pressed(egui::Key::ArrowDown) && self.ctrl_pressed {
                // Ctrl+Down toggles expansion of the currently focused item (same as Ctrl+Up)
                if let Some(focused_idx) = crate::gui::genius_feed::GeniusFeedState::get_focused_index() {
                    if focused_idx < item_count {
                        crate::gui::genius_feed::GeniusFeedState::toggle_item_expansion(focused_idx);
                        handled = true;
                    }
                }
            }
            
            // Ctrl+Enter key to toggle pinning the currently focused item
            if i.key_pressed(egui::Key::Enter) && self.ctrl_pressed {
                if let Some(focused_item) = crate::gui::genius_feed::GeniusFeedState::get_focused_item() {
                    crate::gui::genius_feed::GeniusFeedState::toggle_item_pinned(&focused_item.id);
                    
                    // Request focus back to the input field for the next frame
                    app.display_container_state.request_focus_next_frame = true;
                    
                    handled = true;
                }
            }
        });
        
        handled
    }
}
