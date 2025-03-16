//! Genius Platform API integration.
//! 
//! This module provides integration with the Genius Platform API,
//! allowing the application to query the API and display results.

pub mod genius_api;
pub mod genius_api_bridge;

// Re-export key types for convenience
pub use genius_api::GeniusItem;
pub use genius_api_bridge::GeniusApiBridge;

use std::sync::Mutex;
use lazy_static::lazy_static;

// Create a global instance of GeniusApiBridge
// This allows us to have a single instance that's shared throughout the application
lazy_static! {
    pub static ref GENIUS_API_BRIDGE: Mutex<GeniusApiBridge> = Mutex::new(GeniusApiBridge::new());
}

/// Get a reference to the global GeniusApiBridge
/// 
/// This function provides access to the global GeniusApiBridge instance.
/// It's a convenience wrapper around the GENIUS_API_BRIDGE static.
pub fn get_api_bridge() -> std::sync::MutexGuard<'static, GeniusApiBridge> {
    GENIUS_API_BRIDGE.lock().unwrap()
}
