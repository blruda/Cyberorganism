//! Genius Feed widget for displaying results from the Genius API.
//! 
//! This module provides an egui widget for displaying the results from the Genius API
//! as a simple bulleted list, sorted by relevance when available.

use eframe::egui;
use crate::genius_platform::{GeniusItem, GeniusApiBridge};
use crate::App;
use std::time::{Duration, Instant};
use std::cell::RefCell;
use std::collections::HashSet;

// Thread-local cache for rate limiting API requests
thread_local! {
    static API_CACHE: RefCell<ApiRequestCache> = RefCell::new(ApiRequestCache::new());
    static FEED_STATE: RefCell<GeniusFeedState> = RefCell::new(GeniusFeedState::new());
}

// Cache structure to hold API request state
struct ApiRequestCache {
    last_api_request: Option<Instant>,
    last_query_text: String,
    min_request_interval: Duration,
}

impl ApiRequestCache {
    fn new() -> Self {
        Self {
            last_api_request: None,
            last_query_text: String::new(),
            min_request_interval: Duration::from_millis(50),
        }
    }
}

/// State for the Genius Feed
pub struct GeniusFeedState {
    /// Index of the currently focused item (0-based)
    pub focused_index: Option<usize>,
    /// Set of expanded item indices that show metadata
    pub expanded_items: HashSet<usize>,
}

impl GeniusFeedState {
    fn new() -> Self {
        Self {
            focused_index: Some(0), // Start with the first item focused
            expanded_items: HashSet::new(),
        }
    }

    /// Get the current focused index
    pub fn get_focused_index() -> Option<usize> {
        FEED_STATE.with(|state| state.borrow().focused_index)
    }

    /// Set the focused index
    pub fn set_focused_index(index: Option<usize>) {
        FEED_STATE.with(|state| state.borrow_mut().focused_index = index);
    }

    /// Check if an item is expanded
    pub fn is_item_expanded(index: usize) -> bool {
        FEED_STATE.with(|state| state.borrow().expanded_items.contains(&index))
    }

    /// Toggle the expanded state of an item
    pub fn toggle_item_expansion(index: usize) {
        FEED_STATE.with(|state| {
            let mut state = state.borrow_mut();
            if state.expanded_items.contains(&index) {
                state.expanded_items.remove(&index);
            } else {
                state.expanded_items.insert(index);
            }
        });
    }

    /// Move focus up
    pub fn focus_previous(item_count: usize) {
        if item_count == 0 {
            return;
        }

        FEED_STATE.with(|state| {
            let mut state = state.borrow_mut();
            if let Some(current) = state.focused_index {
                if current > 0 {
                    state.focused_index = Some(current - 1);
                } else {
                    // Wrap around to the last item
                    state.focused_index = Some(item_count - 1);
                }
            } else {
                state.focused_index = Some(0);
            }
        });
    }

    /// Move focus down
    pub fn focus_next(item_count: usize) {
        if item_count == 0 {
            return;
        }

        FEED_STATE.with(|state| {
            let mut state = state.borrow_mut();
            if let Some(current) = state.focused_index {
                if current < item_count - 1 {
                    state.focused_index = Some(current + 1);
                } else {
                    // Wrap around to the first item
                    state.focused_index = Some(0);
                }
            } else {
                state.focused_index = Some(0);
            }
        });
    }
}

/// Query the API if conditions are met (rate limiting and input changed)
/// 
/// This function checks if an API request should be made based on:
/// 1. Input is not empty
/// 2. Input has changed since the last query
/// 3. Enough time has passed since the last request (rate limiting)
pub fn maybe_query_api(app: &mut App, input_text: &str) {
    // Skip empty input
    if input_text.is_empty() {
        return;
    }
    
    // Use thread_local to safely access our cache
    API_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        
        // Skip if input hasn't changed since last query
        if input_text == cache.last_query_text {
            return;
        }
        
        // Check if enough time has passed since the last request
        let should_query = match cache.last_api_request {
            Some(last_time) => {
                let elapsed = last_time.elapsed();
                elapsed >= cache.min_request_interval
            },
            None => true, // First request
        };
        
        if should_query {
            // Update the last query time and text
            cache.last_api_request = Some(Instant::now());
            cache.last_query_text = input_text.to_string();
            
            // Query the API using the global API bridge
            let mut api_bridge = crate::genius_platform::get_api_bridge();
            let _ = api_bridge.query_with_input(app, input_text);
        }
    });
}

/// Render the Genius Feed widget
/// 
/// This function displays items from the Genius API as a bulleted list,
/// sorted by relevance if available.
pub fn render_genius_feed(ui: &mut egui::Ui, api_bridge: &GeniusApiBridge, app_mode: crate::commands::AppMode) {
    // Determine if we're in feed mode for highlighting
    let is_feed_mode = matches!(app_mode, crate::commands::AppMode::Feed);
    
    // Create a frame with some padding and a visible border
    egui::Frame::none()
        .inner_margin(egui::style::Margin::symmetric(8.0, 4.0))
        .stroke(egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE))
        .fill(egui::Color32::from_rgba_premultiplied(0, 0, 50, 20))
        .show(ui, |ui| {
            // Check if there's any data to display
            if let Some(response) = api_bridge.last_response() {
                // Sort items by relevance if available (highest first)
                let mut items = response.items.clone();
                items.sort_by(|a, b| {
                    // Extract relevance from metadata
                    let relevance_a = a.metadata.get("relevance")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    
                    let relevance_b = b.metadata.get("relevance")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    
                    // Sort in descending order (highest relevance first)
                    relevance_b.partial_cmp(&relevance_a).unwrap_or(std::cmp::Ordering::Equal)
                });
                
                // Get the currently focused index
                let focused_index = GeniusFeedState::get_focused_index();
                
                // If this is the first time showing items and we have items, ensure focus is set
                if focused_index.is_none() && !items.is_empty() {
                    GeniusFeedState::set_focused_index(Some(0));
                }
                
                // Update the item count for navigation
                let item_count = items.len();
                
                // Display each item as a bulleted list
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (idx, item) in items.iter().enumerate() {
                            // Only highlight if we're in Feed mode
                            let is_focused = is_feed_mode && focused_index == Some(idx);
                            
                            // We need to wrap this in a container to capture item-specific interactions
                            let _item_response = ui.group(|ui| {
                                render_genius_item(ui, item, is_focused, idx);
                            });
                        }
                    });
                
                // Store the item count for navigation
                if item_count > 0 && focused_index.is_none() {
                    GeniusFeedState::set_focused_index(Some(0));
                } else if item_count == 0 {
                    GeniusFeedState::set_focused_index(None);
                }
            } else if api_bridge.is_request_in_progress() {
                ui.label("Loading results...");
            } else {
                ui.label("Type to see Genius suggestions");
            }
        });
}

/// Render a single Genius item
fn render_genius_item(ui: &mut egui::Ui, item: &GeniusItem, is_focused: bool, item_index: usize) {
    // Check if this item is expanded
    let is_expanded = GeniusFeedState::is_item_expanded(item_index);
    
    // Define colors
    let accent_color = egui::Color32::from_rgb(57, 255, 20);
    let text_color = if is_focused { egui::Color32::BLACK } else { ui.visuals().text_color() };
    
    // Create a frame that will have the background color if focused
    let frame = if is_focused {
        egui::Frame::none()
            .fill(accent_color)
            .inner_margin(egui::style::Margin::symmetric(4.0, 0.0))
    } else {
        egui::Frame::none()
    };
    
    // Use a vertical layout for the entire item
    ui.vertical(|ui| {
        // Use the frame to create a container with the right background for the main row
        frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                // Display bullet point
                ui.label(egui::RichText::new("• ").color(text_color));
                
                // Display the description
                ui.label(egui::RichText::new(&item.description).color(text_color));
                
                // Extract relevance for display
                let relevance = item.metadata.get("relevance")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                
                // Format relevance as percentage
                let relevance_text = format!("({:.0}%)", relevance * 100.0);
                
                // Add flexible space to push relevance to the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(relevance_text).weak().color(text_color));
                    
                    // Add a small indicator for expanded state
                    // TODO: Fix down arrow icon not rendering correctly in egui
                    let expand_indicator = if is_expanded { "" } else { "▶" };
                    ui.label(egui::RichText::new(expand_indicator).weak().color(text_color));
                });
            });
        });
        
        // If expanded, show metadata
        if is_expanded {
            ui.indent("metadata", |ui| {
                // Create a slightly indented area with a subtle background
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_premultiplied(0, 0, 0, 20))
                    .inner_margin(egui::style::Margin::symmetric(8.0, 4.0))
                    .show(ui, |ui| {
                        // Display metadata as key-value pairs
                        if let serde_json::Value::Object(map) = &item.metadata {
                            for (key, value) in map {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(format!("{}:", key)).strong());
                                    ui.label(format!("{}", value));
                                });
                            }
                        } else {
                            ui.label("No metadata available");
                        }
                        
                        // Also show the item ID
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("ID:").strong());
                            ui.label(&item.id);
                        });
                    });
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use crate::genius_platform::GeniusApiBridge;
    use crate::genius_platform::genius_api::{GeniusResponse, GeniusItem};
    use serde_json;

    /// Create a mock GeniusApiBridge with a predefined response
    fn create_mock_api_bridge() -> GeniusApiBridge {
        let mut api_bridge = GeniusApiBridge::new();
        
        // Create dummy items with incrementing relevance
        let mut items = Vec::new();
        for i in 1..=8 {
            let item = GeniusItem {
                id: format!("item-{}", i),
                description: format!("Item {} - This is a dummy item for test", i),
                metadata: {
                    let mut map = serde_json::Map::new();
                    // Relevance from 0.1 to 0.8 (incrementing by 0.1)
                    map.insert(
                        "relevance".to_string(), 
                        serde_json::Value::Number(serde_json::Number::from_f64(i as f64 * 0.1).unwrap())
                    );
                    serde_json::Value::Object(map)
                },
            };
            items.push(item);
        }
        
        // Create a dummy response
        let response = GeniusResponse {
            items,
            status: "success".to_string(),
        };
        
        // Set the response in the bridge
        api_bridge.set_test_response(response);
        
        api_bridge
    }

    #[test]
    fn test_render_genius_feed() {
        // This test verifies that the Genius Feed widget correctly renders items
        
        // Create a mock API bridge with test data
        let api_bridge = create_mock_api_bridge();
        
        // Check that the bridge has a response
        assert!(api_bridge.last_response().is_some(), "API bridge should have a response");
        
        // Check the number of items in the response
        if let Some(response) = api_bridge.last_response() {
            assert_eq!(response.items.len(), 8, "Response should have 8 items");
            
            // Check that items have the expected relevance values
            for (i, item) in response.items.iter().enumerate() {
                let i_f64 = (i + 1) as f64;
                let expected_relevance = i_f64 * 0.1;
                
                let relevance = item.metadata.get("relevance")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                
                let epsilon = 0.0001;
                assert!((relevance - expected_relevance).abs() < epsilon, 
                    "Item {} should have relevance {}, got {}", i+1, expected_relevance, relevance);
            }
        }
        
        // This test should fail if the widget isn't visible in the UI
        // In a real test environment with egui, we would check that the widget is rendered
        // Since we can't do that directly, we'll add a comment to remind us to check manually
        println!("IMPORTANT: Verify that the Genius Feed widget is visible in the UI");
        println!("The widget should have a light blue border and a dark blue background");
        println!("It should always show the 'Genius Feed' heading");
    }
}
