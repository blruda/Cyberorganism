#![allow(dead_code)]
#![allow(unused_variables)]

use super::genius_api::{GeniusApiClient, GeniusApiError, GeniusResponse, GeniusItem};
use crate::App;
use serde_json;

/// Bridge between the application UI and the Genius API
/// 
/// This module handles the communication between the application components
/// and the API, ensuring that data flows correctly in both directions.
pub struct GeniusApiBridge {
    /// The API client used to make requests
    api_client: GeniusApiClient,
    /// The most recent API response
    last_response: Option<GeniusResponse>,
    /// Flag indicating if a request is in progress
    request_in_progress: bool,
}

impl GeniusApiBridge {
    /// Create a new API bridge with default settings
    pub fn new() -> Self {
        Self {
            api_client: GeniusApiClient::new(),
            last_response: None,
            request_in_progress: false,
        }
    }

    /// Create a new API bridge with a custom API client
    pub fn with_client(api_client: GeniusApiClient) -> Self {
        Self {
            api_client,
            last_response: None,
            request_in_progress: false,
        }
    }

    /// Get the input query from the application state
    /// 
    /// This method retrieves the current input text from the DisplayContainerState
    /// which is accessible through the App struct.
    pub fn get_query_from_app(&self, app: &App) -> String {
        app.display_container_state.input_value().to_string()
    }

    /// Execute a query using the input from the application
    /// 
    /// This method takes a reference to the App, extracts the input text,
    /// and sends it to the API. It returns the API response or an error.
    pub fn query_with_app(&mut self, app: &App) -> Result<GeniusResponse, GeniusApiError> {
        let query = self.get_query_from_app(app);
        self.execute_query(&query)
    }

    /// Query the API with specific input text
    /// 
    /// This method takes a reference to the App (for potential future context)
    /// and the input text to query. It returns the API response or an error.
    pub fn query_with_input(&mut self, _app: &App, input: &str) -> Result<GeniusResponse, GeniusApiError> {
        self.execute_query(input)
    }

    /// Execute a query with the given input string
    /// 
    /// This is the core method that actually sends the query to the API
    /// and handles the response.
    fn execute_query(&mut self, query: &str) -> Result<GeniusResponse, GeniusApiError> {
        // Mark that a request is in progress
        self.request_in_progress = true;
        
        // In debug mode, simulate API request with dummy data
        #[cfg(debug_assertions)]
        {
            // Create dummy items with incrementing relevance
            let mut items = Vec::new();
            for i in 1..=8 {
                let item = GeniusItem {
                    id: format!("item-{}", i),
                    description: format!("Result for '{}' - Item {}", query, i),
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
            
            // Store the response and mark request as complete
            self.last_response = Some(response.clone());
            self.request_in_progress = false;
            
            return Ok(response);
        }
        
        // In release mode, use the actual API client
        #[cfg(not(debug_assertions))]
        {
            // TODO: Implement actual API request
            // For now, just return a mock response
            let response = self.api_client.mock_query(query);
            self.last_response = Some(response.clone());
            self.request_in_progress = false;
            
            Ok(response)
        }
    }

    /// Get the descriptions from the last API response
    /// 
    /// This method extracts just the description strings from the API response,
    /// which is the primary data needed by the application.
    pub fn get_descriptions(&self) -> Vec<String> {
        match &self.last_response {
            Some(response) => response.items.iter()
                .map(|item| item.description.clone())
                .collect(),
            None => Vec::new(),
        }
    }

    /// Check if a request is currently in progress
    pub fn is_request_in_progress(&self) -> bool {
        self.request_in_progress
    }

    /// Set a test response directly (for unit testing)
    #[cfg(test)]
    pub fn set_test_response(&mut self, response: GeniusResponse) {
        self.last_response = Some(response);
        self.request_in_progress = false;
    }

    /// Get the last API response, if any
    pub fn last_response(&self) -> Option<&GeniusResponse> {
        self.last_response.as_ref()
    }

    /// Get a reference to the global GeniusApiBridge instance
    pub fn global() -> std::sync::MutexGuard<'static, Self> {
        super::get_api_bridge()
    }
}

/// Factory functions for creating API bridges
pub mod factory {
    use super::*;
    use crate::genius_platform::genius_api::mock;

    /// Create a default API bridge
    pub fn create_default_bridge() -> GeniusApiBridge {
        GeniusApiBridge::new()
    }

    /// Create a mock API bridge for testing
    pub fn create_mock_bridge() -> GeniusApiBridge {
        let mock_client = mock::create_mock_client();
        GeniusApiBridge::with_client(mock_client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genius_api_dummy_data() {
        // Create the API bridge
        let mut api_bridge = GeniusApiBridge::new();
        
        // Access the execute_query method directly
        let test_input = "test query";
        let result = api_bridge.execute_query(test_input);
        
        // Verify the result
        assert!(result.is_ok(), "Query should succeed");
        
        if let Ok(response) = result {
            // Check that we have 8 items
            assert_eq!(response.items.len(), 8, "Should have 8 dummy items");
            
            // Check that the items have incrementing relevance
            for (i, item) in response.items.iter().enumerate() {
                let i_f64 = (i + 1) as f64;
                let expected_relevance = i_f64 * 0.1;
                
                // Extract relevance from metadata
                let relevance = item.metadata.get("relevance")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                
                // Check relevance with a small epsilon for floating point comparison
                let epsilon = 0.0001;
                assert!((relevance - expected_relevance).abs() < epsilon, 
                    "Item {} should have relevance {}, got {}", i+1, expected_relevance, relevance);
                
                // Check that the description contains the query
                assert!(item.description.contains(test_input), 
                    "Item description should contain the query text");
            }
            
            // Check that the status is "success"
            assert_eq!(response.status, "success", "Status should be 'success'");
        }
        
        // Check that the last_response is set
        assert!(api_bridge.last_response().is_some(), "last_response should be set");
        
        // Check that request_in_progress is false
        assert!(!api_bridge.is_request_in_progress(), "request_in_progress should be false");
    }
}
