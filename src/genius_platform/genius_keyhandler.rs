//! Genius Feed input handling module
//! 
//! This module handles keyboard input specifically for the Genius Feed mode.
//! It is separate from the main keyhandler.rs to avoid introducing errors
//! in the complex PKM input handling logic.

use eframe::egui;
use crate::App;
use crate::commands;
use crate::gui::genius_feed;

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
    /// Currently, it only detects Ctrl+Space for mode switching.
    /// 
    /// # Work in Progress
    /// This is a placeholder for future implementation of Genius Feed mode input handling.
    /// Future versions will implement navigation within the Genius Feed results
    /// and keyboard shortcuts for selecting and acting on suggestions.
    pub fn handle_input(&mut self, app: &mut App, ctx: &egui::Context, input_text: &mut String) -> bool {
        let mut handled = false;
        
        // Update modifier keys
        self.update_modifiers(ctx);
        
        // Check if we should query the API based on input changes
        genius_feed::maybe_query_api(app, input_text);
        
        // Check for key presses
        ctx.input(|i| {
            // TODO: Tab key navigation is problematic in egui and causes focus issues.
            // We need to investigate a proper fix for tab navigation in the future.
            // For now, we're using Ctrl+Space instead of Shift+Tab for mode switching.
            
            // Check for Ctrl+Space for mode switching
            if i.key_pressed(egui::Key::Space) && self.ctrl_pressed {
                app.app_mode = commands::toggle_app_mode(app, app.app_mode);
                handled = true;
            }
            
            // Future: Add handling for navigation, selection, and actions on Genius Feed results
        });
        
        handled
    }
}
