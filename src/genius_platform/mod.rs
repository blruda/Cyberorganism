//! Genius Platform API integration.
//! 
//! This module provides integration with the Genius Platform API,
//! allowing the application to query the API and display results.

pub mod genius_api;
pub mod genius_api_bridge;
pub mod genius_keyhandler;

// Re-export key types for convenience
pub use genius_api::GeniusItem;
pub use genius_api_bridge::GeniusApiBridge;

use std::sync::Mutex;
use lazy_static::lazy_static;
use std::env;

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

/// Initialize the Genius API with credentials from environment variables
///
/// This function attempts to load the API key and organization ID from
/// environment variables and configure the API bridge with them.
///
/// Environment variables:
/// - GENIUS_API_KEY: The API key for authenticating with the Genius API
/// - GENIUS_ORGANIZATION_ID: The organization ID for the Genius API
///
/// Returns true if the API was successfully configured, false otherwise.
pub fn initialize_from_env() -> bool {
    println!("[DEBUG] Initializing Genius API from environment variables");
    
    let api_key = env::var("GENIUS_API_KEY").ok();
    let org_id = env::var("GENIUS_ORGANIZATION_ID").ok();
    
    println!("[DEBUG] GENIUS_API_KEY present: {}", api_key.is_some());
    println!("[DEBUG] GENIUS_ORGANIZATION_ID present: {}", org_id.is_some());
    
    if let (Some(api_key), Some(org_id)) = (api_key.clone(), org_id.clone()) {
        println!("[DEBUG] Configuring API bridge with API key and organization ID");
        println!("[DEBUG] Organization ID: '{}'", org_id);
        
        let mut bridge = get_api_bridge();
        bridge.configure(&api_key, &org_id);
        true
    } else {
        println!("[DEBUG] Missing environment variables for Genius API");
        false
    }
}

/// Initialize the Genius API with the provided credentials
///
/// This function configures the API bridge with the given API key and organization ID.
pub fn initialize(api_key: &str, organization_id: &str) {
    let mut bridge = get_api_bridge();
    bridge.configure(api_key, organization_id);
}
