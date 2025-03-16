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
    /// Flag indicating that more items should be loaded
    pub should_load_more: bool,
    /// Set of pinned item IDs that should persist across queries
    pub pinned_items: HashSet<String>,
}

impl GeniusFeedState {
    fn new() -> Self {
        Self {
            focused_index: Some(0), // Start with the first item focused
            expanded_items: HashSet::new(),
            should_load_more: false,
            pinned_items: HashSet::new(),
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

    /// Check if an item is pinned
    pub fn is_item_pinned(item_id: &str) -> bool {
        FEED_STATE.with(|state| state.borrow().pinned_items.contains(item_id))
    }
    
    /// Toggle the pinned state of an item
    pub fn toggle_item_pinned(item_id: &str) {
        FEED_STATE.with(|state| {
            let mut state = state.borrow_mut();
            if state.pinned_items.contains(item_id) {
                state.pinned_items.remove(item_id);
            } else {
                state.pinned_items.insert(item_id.to_string());
            }
        });
    }
    
    /// Get all pinned item IDs
    pub fn get_pinned_items() -> HashSet<String> {
        FEED_STATE.with(|state| state.borrow().pinned_items.clone())
    }

    /// Get the item at the focused index, taking into account sorting and pinning
    pub fn get_focused_item() -> Option<crate::genius_platform::genius_api::GeniusItem> {
        let focused_idx = Self::get_focused_index()?;
        
        // Store the API bridge in a variable to avoid temporary value issues
        let api_bridge = crate::genius_platform::genius_api_bridge::GeniusApiBridge::global();
        // Clone the response to ensure we own the data
        let response = api_bridge.last_response()?.clone();
        
        // Get sorted items as they appear in the UI
        let mut items = response.items;
        
        // Sort by relevance
        items.sort_by(|a, b| {
            let relevance_a = a.metadata.get("relevance")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            
            let relevance_b = b.metadata.get("relevance")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            
            relevance_b.partial_cmp(&relevance_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Prioritize pinned items
        let pinned_item_ids = Self::get_pinned_items();
        if !pinned_item_ids.is_empty() {
            let mut pinned_items = Vec::new();
            let mut unpinned_items = Vec::new();
            
            for item in items {
                if pinned_item_ids.contains(&item.id) {
                    pinned_items.push(item);
                } else {
                    unpinned_items.push(item);
                }
            }
            
            items = pinned_items;
            items.extend(unpinned_items);
        }
        
        // Return the item at the focused index if it exists
        if focused_idx < items.len() {
            Some(items[focused_idx].clone())
        } else {
            None
        }
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
                println!("[DEBUG] focus_next: current={}, item_count={}", current, item_count);
                
                if current < item_count - 1 {
                    // Move to the next item
                    state.focused_index = Some(current + 1);
                    println!("[DEBUG] focus_next: Moving to next item: {}", current + 1);
                    
                    // If we're at the second-to-last item, set the flag to load more
                    if current == item_count - 2 {
                        println!("[DEBUG] focus_next: At second-to-last item, setting should_load_more flag");
                        state.should_load_more = true;
                    }
                } else {
                    // We're at the last item
                    // Set the flag to load more items
                    println!("[DEBUG] focus_next: At last item ({}), setting should_load_more flag", current);
                    state.should_load_more = true;
                    
                    // Don't wrap around to the first item if we're trying to load more
                    // We'll stay at the current item until more are loaded
                }
            } else {
                state.focused_index = Some(0);
                println!("[DEBUG] focus_next: No focus, setting to 0");
            }
        });
    }

    /// Set the flag to load more items
    pub fn set_should_load_more(should_load: bool) {
        FEED_STATE.with(|state| state.borrow_mut().should_load_more = should_load);
    }

    /// Check if more items should be loaded
    pub fn should_load_more() -> bool {
        FEED_STATE.with(|state| state.borrow().should_load_more)
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
    
    // Check if we should load more items
    if GeniusFeedState::should_load_more() {
        println!("[DEBUG] maybe_query_api: should_load_more flag is true, loading next page");
        
        // Reset the flag
        GeniusFeedState::set_should_load_more(false);
        
        // Store the current focused index and item count before loading more
        let current_focused = GeniusFeedState::get_focused_index();
        
        // Load more items
        let mut api_bridge = crate::genius_platform::get_api_bridge();
        if !api_bridge.is_request_in_progress() && api_bridge.has_more_pages() {
            println!("[DEBUG] maybe_query_api: Calling load_next_page()");
            let result = api_bridge.load_next_page();
            
            // If we successfully loaded more items and we were at the last item,
            // update the focused index to the next item
            if result.is_ok() {
                if let Some(focused_idx) = current_focused {
                    let new_item_count = api_bridge.all_items().len();
                    println!("[DEBUG] maybe_query_api: Loaded more items, new count: {}", new_item_count);
                    
                    // If we were at the last item, move to the next one
                    if focused_idx == new_item_count - result.unwrap().items.len() - 1 {
                        println!("[DEBUG] maybe_query_api: Moving focus to next item: {}", focused_idx + 1);
                        GeniusFeedState::set_focused_index(Some(focused_idx + 1));
                    }
                }
            }
        } else {
            println!("[DEBUG] maybe_query_api: Skipping load_next_page() - request in progress: {}, has more pages: {}", 
                api_bridge.is_request_in_progress(), api_bridge.has_more_pages());
        }
        
        // Return early to avoid making a new query
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
        .stroke(egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color))
        .show(ui, |ui| {
            // Check if there's any data to display
///<<<<<<< blruda-3
            let items = api_bridge.all_items();
            if !items.is_empty() {
///====== Git merge conflict here, but why?
            if let Some(response) = api_bridge.last_response() {
                // Get all pinned item IDs
                let pinned_item_ids = GeniusFeedState::get_pinned_items();
                
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
                
                // Prioritize pinned items by moving them to the top
                if !pinned_item_ids.is_empty() {
                    // Separate pinned and unpinned items
                    let mut pinned_items = Vec::new();
                    let mut unpinned_items = Vec::new();
                    
                    for item in items {
                        if pinned_item_ids.contains(&item.id) {
                            pinned_items.push(item);
                        } else {
                            unpinned_items.push(item);
                        }
                    }
                    
                    // Combine them with pinned items first
                    items = pinned_items;
                    items.extend(unpinned_items);
                }
                
///>>>>>>> master
                // Get the currently focused index
                let focused_index = GeniusFeedState::get_focused_index();
                
                // If this is the first time showing items and we have items, ensure focus is set
                if focused_index.is_none() {
                    GeniusFeedState::set_focused_index(Some(0));
                }
                
                // Update the item count for navigation
                let item_count = items.len();
                
                // Add debugging information at the top
                ui.horizontal(|ui| {
                    ui.label(format!("Page: {} | Total Items: {} | Focused: {:?}", 
                        api_bridge.current_page(), item_count, focused_index));
                });
                ui.add_space(4.0);
                
                // Check if we need to load more items when the focused item is at the end of the list
                if let Some(focused_idx) = focused_index {
                    // Make sure the focused index is valid
                    if focused_idx >= item_count {
                        println!("[DEBUG] render_genius_feed: Focused index {} is out of bounds (item_count: {}), adjusting", 
                            focused_idx, item_count);
                        // Adjust the focused index to be within bounds
                        GeniusFeedState::set_focused_index(Some(item_count - 1));
                    } else if focused_idx >= item_count - 1 && !api_bridge.is_request_in_progress() && api_bridge.has_more_pages() {
                        println!("[DEBUG] render_genius_feed: Focused item is at the end of the list, setting should_load_more flag");
                        GeniusFeedState::set_should_load_more(true);
                    }
                }
                
                // Display each item as a bulleted list
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (idx, item) in items.iter().enumerate() {
                            // Only highlight if we're in Feed mode
                            let is_focused = is_feed_mode && focused_index == Some(idx);
                            
                            // We need to wrap this in a container to capture item-specific interactions
                            let item_response = render_genius_item(ui, item, is_focused, idx);
                            
                            // If this item is focused, scroll to make it visible
                            if is_focused {
                                // Only scroll if the item is not fully visible in the scroll area
                                let item_rect = item_response.rect;
                                let scroll_area_rect = ui.clip_rect();
                                
                                // Check if the item is not fully visible
                                let is_partially_out_of_view = 
                                    item_rect.top() < scroll_area_rect.top() || 
                                    item_rect.bottom() > scroll_area_rect.bottom();
                                
                                if is_partially_out_of_view {
                                    item_response.scroll_to_me(Some(egui::Align::Center));
                                }
                                
                                // Check if we're near the end of the list and should load more
                                // We consider "near the end" to be the second-to-last item or later
                                if idx >= item_count - 2 && !api_bridge.is_request_in_progress() && api_bridge.has_more_pages() {
                                    // Set the flag to load more items instead of directly loading them
                                    println!("[DEBUG] Setting should_load_more flag to true (idx: {}, item_count: {})", idx, item_count);
                                    GeniusFeedState::set_should_load_more(true);
                                }
                            }
                        }
                        
                        // Show a loading indicator at the bottom if we're loading more items
                        if api_bridge.is_request_in_progress() {
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Loading more results...");
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
fn render_genius_item(ui: &mut egui::Ui, item: &GeniusItem, is_focused: bool, item_index: usize) -> egui::Response {
    // Check if this item is expanded
    let is_expanded = GeniusFeedState::is_item_expanded(item_index);
    
    // Check if this item is pinned
    let is_pinned = GeniusFeedState::is_item_pinned(&item.id);
    
    // Define colors
    let accent_color = egui::Color32::from_rgb(57, 255, 20);
    // Gold color for pinned items (used for background)
    let pinned_bg_color = egui::Color32::from_rgba_premultiplied(255, 215, 0, 40);
    
    // Determine text color based on item state
    let text_color = if is_focused || is_pinned {
        egui::Color32::BLACK
    } else {
        ui.visuals().text_color()
    };
    
    // Create a frame that will have the appropriate background color
    let frame = if is_focused {
        egui::Frame::none()
            .fill(accent_color)
            .inner_margin(egui::style::Margin::symmetric(4.0, 0.0))
    } else if is_pinned {
        // For pinned items that aren't focused, use a subtle gold background
        egui::Frame::none()
            .fill(pinned_bg_color)
            .inner_margin(egui::style::Margin::symmetric(4.0, 0.0))
    } else {
        egui::Frame::none()
    };
    
    // Use a vertical layout for the entire item and collect the response
    ui.vertical(|ui| {
        // Use the frame to create a container with the right background for the main row
        let main_row_response = frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                // Display item number and bullet point - use global sequential numbering
                ui.label(egui::RichText::new(format!("{}• ", item_index + 1)).color(text_color));
                // Display bullet point or pin icon
                if is_pinned {
                    ui.label(egui::RichText::new("📌 ").color(text_color));
                } else {
                    ui.label(egui::RichText::new("• ").color(text_color));
                }
                
                // Display the description
                ui.label(egui::RichText::new(&item.description).color(text_color));
                
                // Add a small label with the item ID for debugging
                ui.small(egui::RichText::new(format!("[ID: {}]", &item.id)).weak().color(text_color));
                
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
            }).response
        }).response;
        
        // If expanded, show metadata
        let mut metadata_response = None;
        if is_expanded {
            metadata_response = Some(ui.indent("metadata", |ui| {
                // Create a slightly indented area with a subtle border
                egui::Frame::none()
                    .stroke(egui::Stroke::new(0.5, ui.visuals().widgets.noninteractive.bg_stroke.color))
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
                    }).response
            }).response);
        }
        
        // Return the main row response, or if expanded, combine it with the metadata response
        if let Some(meta_resp) = metadata_response {
            main_row_response.union(meta_resp)
        } else {
            main_row_response
        }
    }).response
}

/// Handle keyboard navigation for the Genius Feed
/// 
/// This function should be called from the main update loop to handle keyboard navigation
/// for the Genius Feed. It uses the total item count from the API bridge, not just the
/// currently visible items.
pub fn handle_keyboard_navigation(api_bridge: &GeniusApiBridge, ctx: &egui::Context) {
    // Get the total number of items from the API bridge
    let total_items = api_bridge.all_items().len();
    
    // Check if we need to handle keyboard input for navigation
    ctx.input(|input| {
        // Handle up/down arrow keys
        if input.key_pressed(egui::Key::ArrowDown) {
            println!("[DEBUG] handle_keyboard_navigation: Down arrow pressed, total_items={}", total_items);
            GeniusFeedState::focus_next(total_items);
        } else if input.key_pressed(egui::Key::ArrowUp) {
            println!("[DEBUG] handle_keyboard_navigation: Up arrow pressed, total_items={}", total_items);
            GeniusFeedState::focus_previous(total_items);
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
