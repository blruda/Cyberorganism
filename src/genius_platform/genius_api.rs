#![allow(dead_code)]
#![allow(unused_variables)]

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::time::Duration;
use uuid;

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
            base_url: "https://api.genius.com".to_string(),
            api_key: None,
            timeout: Duration::from_secs(10),
            organization_id: String::new(),
            session_id: uuid::Uuid::new_v4().to_string(),
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
            session_id: uuid::Uuid::new_v4().to_string(),
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

    /// Get the server URL for API requests
    fn get_server_url(&self) -> String {
        format!("{}/hackathon/{}/feed/{}", 
            self.base_url, 
            self.organization_id,
            self.session_id
        )
    }

    /// Query the API synchronously
    pub fn query_sync(&self, input: &str) -> Result<GeniusResponse, GeniusApiError> {
        // If we're in debug mode or missing configuration, return mock data
        if cfg!(debug_assertions) || self.api_key.is_none() || self.organization_id.is_empty() {
            return Ok(self.mock_query(input));
        }

        // Create the request body
        let body = serde_json::json!({
            "text": input,
            "page": 1,
            "batch_count": 10
        });

        // Create the client with timeout
        let client = match reqwest::blocking::ClientBuilder::new()
            .timeout(self.timeout)
            .build() {
                Ok(client) => client,
                Err(e) => return Err(GeniusApiError::NetworkError(e.to_string())),
            };

        // Make the API request
        let response = match client
            .post(&self.get_server_url())
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key.as_ref().unwrap_or(&String::new())))
            .json(&body)
            .send() {
                Ok(response) => response,
                Err(e) => return Err(GeniusApiError::NetworkError(e.to_string())),
            };

        // Check if the request was successful
        if !response.status().is_success() {
            return Err(GeniusApiError::ApiError(format!(
                "API returned error status: {}", response.status()
            )));
        }

        // Parse the response
        let text = match response.text() {
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
        let items = match serde_json::from_value::<Vec<GeniusItem>>(cards.clone()) {
            Ok(items) => items,
            Err(e) => {
                // If we can't parse directly, try to convert each card manually
                let mut items = Vec::new();
                if let Some(cards_array) = cards.as_array() {
                    for (i, card) in cards_array.iter().enumerate() {
                        // Extract the description or use a default
                        let description = card.get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&format!("Item {}", i+1))
                            .to_string();
                        
                        // Create a GeniusItem
                        let item = GeniusItem {
                            id: format!("item-{}", i+1),
                            description,
                            metadata: card.clone(),
                        };
                        items.push(item);
                    }
                }
                
                if items.is_empty() {
                    return Err(GeniusApiError::ParseError(e.to_string()));
                }
                
                items
            }
        };

        // Create and return the response
        Ok(GeniusResponse {
            items,
            status: "success".to_string(),
        })
    }
    
    /// Create a mock response for testing and development
    pub fn mock_query(&self, query: &str) -> GeniusResponse {
        // Create dummy items with incrementing relevance
        let mut items = Vec::new();
        for i in 1..=8 {
            let item = GeniusItem {
                id: format!("item-{}", i),
                description: format!("Item {} - This is a mock item for query: '{}'", i, query),
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
        
        // Create a mock response
        GeniusResponse {
            items,
            status: "success".to_string(),
        }
    }

    /// Query the API asynchronously
    pub async fn query(&self, input: &str) -> Result<GeniusResponse, GeniusApiError> {
        // If we're in debug mode or missing configuration, return mock data
        if cfg!(debug_assertions) || self.api_key.is_none() || self.organization_id.is_empty() {
            return Ok(self.mock_query(input));
        }

        // Create the request body
        let body = serde_json::json!({
            "text": input,
            "page": 1,
            "batch_count": 10
        });

        // Create the client with timeout
        let client = match reqwest::ClientBuilder::new()
            .timeout(self.timeout)
            .build() {
                Ok(client) => client,
                Err(e) => return Err(GeniusApiError::NetworkError(e.to_string())),
            };

        // Make the API request
        let response = match client
            .post(&self.get_server_url())
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key.as_ref().unwrap_or(&String::new())))
            .json(&body)
            .send()
            .await {
                Ok(response) => response,
                Err(e) => return Err(GeniusApiError::NetworkError(e.to_string())),
            };

        // Check if the request was successful
        if !response.status().is_success() {
            return Err(GeniusApiError::ApiError(format!(
                "API returned error status: {}", response.status()
            )));
        }

        // Parse the response
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
        let items = match serde_json::from_value::<Vec<GeniusItem>>(cards.clone()) {
            Ok(items) => items,
            Err(e) => {
                // If we can't parse directly, try to convert each card manually
                let mut items = Vec::new();
                if let Some(cards_array) = cards.as_array() {
                    for (i, card) in cards_array.iter().enumerate() {
                        // Extract the description or use a default
                        let description = card.get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&format!("Item {}", i+1))
                            .to_string();
                        
                        // Create a GeniusItem
                        let item = GeniusItem {
                            id: format!("item-{}", i+1),
                            description,
                            metadata: card.clone(),
                        };
                        items.push(item);
                    }
                }
                
                if items.is_empty() {
                    return Err(GeniusApiError::ParseError(e.to_string()));
                }
                
                items
            }
        };

        // Create and return the response
        Ok(GeniusResponse {
            items,
            status: "success".to_string(),
        })
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

    /// Filter items by a relevance threshold (if available in metadata)
    pub fn filter_by_relevance(response: &GeniusResponse, threshold: f64) -> Vec<GeniusItem> {
        response.items.iter()
            .filter(|item| {
                if let Some(relevance) = item.metadata.get("relevance") {
                    if let Some(relevance) = relevance.as_f64() {
                        return relevance >= threshold;
                    }
                }
                false
            })
            .cloned()
            .collect()
    }
}
