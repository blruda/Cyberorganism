#![allow(dead_code)]
#![allow(unused_variables)]

//! Genius API client implementation
//!
//! # Schema Update Instructions
//!
//! When the actual Genius API schema becomes available, the following components
//! in this file will need to be updated:
//!
//! 1. **Data Structures**:
//!    - Update `GeniusItem` struct to match the actual response item format
//!    - Update `GeniusResponse` struct to match the actual response envelope
//!    - Add any additional data structures needed for the API
//!
//! 2. **Request Construction**:
//!    - In the `query_sync` method, update the request body JSON (around line 125)
//!    - Ensure all required fields are included in the request
//!    - Update headers if needed (currently using Bearer token authentication)
//!
//! 3. **Response Parsing**:
//!    - Ensure the response parsing logic correctly handles the actual API format
//!    - Update error handling for any API-specific error responses
//!
//! 4. **Mock Data**:
//!    - Update the `mock_query` method to return data that matches the structure
//!      of the real API responses for testing purposes
//!
//! The rest of the application interacts with this API through the `GeniusApiBridge`,
//! so changes should be contained to this file and won't affect other parts of the
//! application as long as the public interface remains consistent.

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::time::Duration;
use uuid::Uuid;

/// Represents an item returned from the Genius API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeniusItem {
    /// Unique identifier for the item
    pub id: String,
    /// Description text for the item
    pub description: String,
    /// Additional metadata as a JSON object
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Response from the Genius API containing multiple items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeniusResponse {
    /// List of items returned from the API
    pub items: Vec<GeniusItem>,
    /// Status of the response
    pub status: String,
}

/// Error types that can occur during API operations
#[derive(Debug)]
pub enum GeniusApiError {
    /// Error occurred during network request
    NetworkError(String),
    /// Error parsing the response
    ParseError(String),
    /// API returned an error
    ApiError(String),
    /// Other unexpected errors
    Other(String),
}

impl std::fmt::Display for GeniusApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NetworkError(msg) => write!(f, "Network error: {}", msg),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Self::ApiError(msg) => write!(f, "API error: {}", msg),
            Self::Other(msg) => write!(f, "Other error: {}", msg),
        }
    }
}

impl Error for GeniusApiError {}

/// Client for interacting with the Genius API
pub struct GeniusApiClient {
    base_url: String,
    api_key: Option<String>,
    timeout: Duration,
    organization_id: String,
    session_id: String,
}

impl GeniusApiClient {
    /// Create a new API client with default settings
    pub fn new() -> Self {
        Self {
            base_url: "https://app.productgenius.io".to_string(),
            api_key: None,
            timeout: Duration::from_secs(10),
            organization_id: String::new(),
            session_id: Uuid::new_v4().to_string(),
        }
    }

    /// Create a new API client with custom configuration
    pub fn with_config(
        base_url: String,
        api_key: Option<String>,
        timeout: Duration,
        organization_id: String,
    ) -> Self {
        Self {
            base_url,
            api_key,
            timeout,
            organization_id,
            session_id: Uuid::new_v4().to_string(),
        }
    }

    /// Set the API key
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    /// Set the organization ID
    pub fn with_organization_id(mut self, organization_id: String) -> Self {
        self.organization_id = organization_id;
        self
    }

    /// Get the base URL for the API
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the timeout duration
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Get the server URL for API requests
    fn get_server_url(&self) -> String {
        format!("{}/hackathon/{}/feed/{}", 
            self.base_url, 
            self.organization_id,
            self.session_id
        )
    }

    /// Query the API synchronously with a specific page number
    pub fn query_sync_with_page(&self, input: &str, page: usize) -> Result<GeniusResponse, GeniusApiError> {
        // When mock-api feature is explicitly enabled, always use mock data
        #[cfg(feature = "mock-api")]
        {
            println!("[DEBUG] Using mock data due to mock-api feature");
            return Ok(self.mock_query(input));
        }

        // In normal mode, try to use real API but fall back to mock if no API key or organization ID
        #[cfg(not(feature = "mock-api"))]
        {
            // If no API key is provided or it's empty, or organization ID is empty, fall back to mock data
            if self.api_key.is_none() || 
               self.api_key.as_ref().map_or(true, |k| k.trim().is_empty()) ||
               self.organization_id.is_empty() {
                println!("[DEBUG] Using mock data due to missing API key or organization ID");
                println!("[DEBUG] API key present: {}", self.api_key.is_some());
                println!("[DEBUG] Organization ID: '{}'", self.organization_id);
                return Ok(self.mock_query(input));
            }
            
            // API key is available, proceed with real API request
            let api_key = self.api_key.as_ref().unwrap();
            let server_url = self.get_server_url();
            
            println!("[DEBUG] Sending API request to: {}", server_url);
            println!("[DEBUG] Query text: '{}' (page {})", input, page);
            
            // Create the request client with timeout
            let client = match reqwest::blocking::Client::builder()
                .timeout(self.timeout)
                .build() {
                    Ok(client) => client,
                    Err(e) => {
                        println!("[DEBUG] Failed to build HTTP client: {}", e);
                        return Err(GeniusApiError::NetworkError(e.to_string()));
                    }
                };
            
            // Prepare the request body based on the genius-hackathon-skeleton implementation
            let request_body = serde_json::json!({
                "search_prompt": input,
                "page": page,
                "batch_count": 10
            });
            
            // Comment out detailed request body logging
            // println!("[DEBUG] Request body: {}", serde_json::to_string_pretty(&request_body).unwrap_or_default());
            
            // Debug the full request details - commented out for reduced output
            let auth_header = format!("Bearer {}", api_key);
            // println!("[DEBUG] Full request details:");
            // println!("[DEBUG] URL: {}", server_url);
            // println!("[DEBUG] Authorization header: {}", auth_header);
            // println!("[DEBUG] Content-Type: application/json");
            
            // Execute the request
            let response = match client
                .post(&server_url)
                .header("Authorization", auth_header)
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send() {
                    Ok(resp) => {
                        println!("[DEBUG] Received response with status: {}", resp.status());
                        
                        // Comment out detailed response headers logging
                        // println!("[DEBUG] Response headers:");
                        // for (name, value) in resp.headers() {
                        //     println!("[DEBUG]   {}: {}", name, value.to_str().unwrap_or("(invalid header value)"));
                        // }
                        
                        resp
                    },
                    Err(e) => {
                        println!("[DEBUG] Request failed: {}", e);
                        return Err(GeniusApiError::NetworkError(e.to_string()));
                    }
                };
            
            // Check the response status
            if !response.status().is_success() {
                let error_msg = format!("API returned error status: {}", response.status());
                println!("[DEBUG] {}", error_msg);
                
                // Try to get the response body for more error details
                match response.text() {
                    Ok(error_body) => {
                        // Comment out detailed error body logging
                        // println!("[DEBUG] Error response body: {}", error_body);
                        return Err(GeniusApiError::ApiError(format!("{}: {}", error_msg, error_body)));
                    },
                    Err(_) => {
                        return Err(GeniusApiError::ApiError(error_msg));
                    }
                }
            }
            
            // Parse the response text first
            let text = match response.text() {
                Ok(text) => {
                    // Comment out full response text logging
                    // println!("[DEBUG] Response text: {}", text);
                    text
                },
                Err(e) => {
                    println!("[DEBUG] Failed to read response text: {}", e);
                    return Err(GeniusApiError::NetworkError(e.to_string()));
                }
            };
            
            // Parse the JSON response
            let payload: serde_json::Value = match serde_json::from_str(&text) {
                Ok(payload) => payload,
                Err(e) => {
                    println!("[DEBUG] Failed to parse JSON: {}", e);
                    return Err(GeniusApiError::ParseError(e.to_string()));
                }
            };
            
            // Extract the cards from the response
            let cards = match payload.get("cards") {
                Some(cards) => cards,
                None => {
                    println!("[DEBUG] No 'cards' field in response");
                    return Err(GeniusApiError::ParseError("No cards in response".to_string()));
                }
            };
            
            // Convert the cards to GeniusItems
            let items = match self.convert_cards_to_items(cards) {
                Ok(items) => {
                    println!("[DEBUG] Successfully converted {} cards to GeniusItems", items.len());
                    items
                },
                Err(e) => {
                    println!("[DEBUG] Failed to convert cards to GeniusItems: {}", e);
                    return Err(e);
                }
            };
            
            // Create and return the response
            Ok(GeniusResponse {
                items,
                status: "success".to_string(),
            })
        }
    }

    /// Query the API synchronously (page 1)
    /// 
    /// This is a wrapper around query_sync_with_page for backward compatibility
    pub fn query_sync(&self, input: &str) -> Result<GeniusResponse, GeniusApiError> {
        self.query_sync_with_page(input, 1)
    }

    /// Convert cards from the API response to GeniusItems
    fn convert_cards_to_items(&self, cards: &serde_json::Value) -> Result<Vec<GeniusItem>, GeniusApiError> {
        let mut items = Vec::new();
        
        if let Some(cards_array) = cards.as_array() {
            for (i, card) in cards_array.iter().enumerate() {
                // Extract the text from product.body or use a default
                let description = card.get("product")
                    .and_then(|product| product.get("body"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&format!("Item {}", i+1))
                    .to_string();
                
                // Extract the ID or generate one
                let id = card.get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&format!("item-{}", i+1))
                    .to_string();
                
                // Create a GeniusItem
                let item = GeniusItem {
                    id,
                    description,
                    metadata: {
                        let map = card.clone();
                        
                        map
                    },
                };
                
                items.push(item);
            }
        }
        
        if items.is_empty() {
            return Err(GeniusApiError::ParseError("Failed to parse cards from response".to_string()));
        }
        
        Ok(items)
    }
    
    /// Create a mock response for testing and development
    pub fn mock_query(&self, query: &str) -> GeniusResponse {
        // Create dummy items with simple numeric IDs and static+dynamic descriptions
        let mut items = Vec::new();
        
        // Static descriptions for each item
        let static_descriptions = [
            "Implement authentication system",
            "Create database schema",
            "Design user interface",
            "Write documentation",
            "Set up CI/CD pipeline",
            "Optimize performance",
            "Fix security vulnerabilities",
            "Add analytics tracking",
        ];
        
        for i in 1..=8 {
            let item = GeniusItem {
                // Simple numeric IDs for easy debugging
                id: i.to_string(),
                // Combine static description with dynamic query information
                description: format!("Item {}: {} (query: '{}')", i, static_descriptions[i-1], query),
                metadata: serde_json::json!({}),
            };
            items.push(item);
        }
        
        // Create a mock response
        GeniusResponse {
            items,
            status: "success".to_string(),
        }
    }

    /// Query the API asynchronously
    pub async fn query(&self, input: &str) -> Result<GeniusResponse, GeniusApiError> {
        // When mock-api feature is explicitly enabled, always use mock data
        #[cfg(feature = "mock-api")]
        {
            return Ok(self.mock_query(input));
        }

        // In normal mode, try to use real API but fall back to mock if no API key or organization ID
        #[cfg(not(feature = "mock-api"))]
        {
            // If no API key is provided or it's empty, or organization ID is empty, fall back to mock data
            if self.api_key.is_none() || 
               self.api_key.as_ref().map_or(true, |k| k.trim().is_empty()) ||
               self.organization_id.is_empty() {
                return Ok(self.mock_query(input));
            }
            
            // API key is available, proceed with real API request
            let api_key = self.api_key.as_ref().unwrap();
            
            // Create the request client with timeout
            let client = match reqwest::Client::builder()
                .timeout(self.timeout)
                .build() {
                    Ok(client) => client,
                    Err(e) => return Err(GeniusApiError::NetworkError(e.to_string())),
                };
            
            // Prepare the request body based on the genius-hackathon-skeleton implementation
            let request_body = serde_json::json!({
                "search_prompt": input,
                "page": 1,
                "batch_count": 10
            });
            
            // Execute the request
            let response = match client
                .post(&self.get_server_url())
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await {
                    Ok(resp) => resp,
                    Err(e) => return Err(GeniusApiError::NetworkError(e.to_string())),
                };
            
            // Check the response status
            if !response.status().is_success() {
                return Err(GeniusApiError::ApiError(
                    format!("API returned error status: {}", response.status())
                ));
            }
            
            // Parse the response text first
            let text = match response.text().await {
                Ok(text) => text,
                Err(e) => return Err(GeniusApiError::NetworkError(e.to_string())),
            };
            
            // Parse the JSON response
            let payload: serde_json::Value = match serde_json::from_str(&text) {
                Ok(payload) => payload,
                Err(e) => return Err(GeniusApiError::ParseError(e.to_string())),
            };
            
            // Extract the cards from the response
            let cards = match payload.get("cards") {
                Some(cards) => cards,
                None => return Err(GeniusApiError::ParseError("No cards in response".to_string())),
            };
            
            // Convert the cards to GeniusItems
            let items = match self.convert_cards_to_items(cards) {
                Ok(items) => items,
                Err(e) => return Err(e),
            };
            
            // Create and return the response
            Ok(GeniusResponse {
                items,
                status: "success".to_string(),
            })
        }
    }
}

/// Module containing mock implementations for testing
pub mod mock {
    use super::*;

    /// Creates a mock API client that returns predefined responses
    pub fn create_mock_client() -> GeniusApiClient {
        GeniusApiClient::new()
    }

    /// Creates a mock response with the given items
    pub fn create_mock_response(items: Vec<GeniusItem>) -> GeniusResponse {
        GeniusResponse {
            items,
            status: "success".to_string(),
        }
    }
}

/// Utility functions for working with API responses
pub mod utils {
    use super::*;

    /// Extract descriptions from a list of items
    pub fn extract_descriptions(response: &GeniusResponse) -> Vec<String> {
        response.items.iter()
            .map(|item| item.description.clone())
            .collect()
    }
}
