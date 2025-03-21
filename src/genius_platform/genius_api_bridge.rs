#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use super::genius_api::{GeniusApiClient, GeniusApiError, GeniusResponse, GeniusItem};
use crate::App;
use serde_json;

/// Bridge between the application UI and the Genius API
/// 
/// This module handles the communication between the application components
/// and the API, ensuring that data flows correctly in both directions.
///
/// # API Schema (PLACEHOLDER)
///
/// IMPORTANT: The schema below is a hypothetical example and MUST be replaced
/// with the actual schema when it becomes available. This is only a placeholder
/// to illustrate the expected structure.
///
/// ## Example Request Format (TO BE REPLACED)
/// ```json
/// {
///   "query": "User input text",
///   "max_results": 5,
///   "filters": {
///     "type": "suggestion",
///     "min_relevance": 0.5
///   }
/// }
/// ```
///
/// ## Example Response Format (TO BE REPLACED)
/// ```json
/// {
///   "status": "success",
///   "items": [
///     {
///       "id": "item-123",
///       "description": "A suggestion from the Genius API",
///       "metadata": {
///         "relevance": 0.95,
///         "source": "knowledge-base",
///         "category": "suggestion"
///       }
///     },
///     {
///       "id": "item-456",
///       "description": "Another suggestion from the API",
///       "metadata": {
///         "relevance": 0.82,
///         "source": "web-search",
///         "category": "fact"
///       }
///     }
///   ]
/// }
/// ```
///
/// # Integration Notes
///
/// This bridge is the primary interface between the application and the Genius API.
/// All API communication should go through this bridge to ensure proper isolation.
/// 
/// TODO: When the actual JSON schema is finalized, update:
/// 1. This documentation with the correct request/response formats
/// 2. The GeniusApiClient in genius_api.rs to match the schema
/// 3. Ensure the bridge methods properly transform data between the app and API
pub struct GeniusApiBridge {
    /// The API client used to make requests
    api_client: GeniusApiClient,
    /// The most recent API response
    last_response: Option<GeniusResponse>,
    /// Flag indicating if a request is in progress
    request_in_progress: bool,
    /// Current page number (1-based)
    current_page: usize,
    /// Current query text
    current_query: String,
    /// All items loaded so far (across all pages)
    all_items: Vec<GeniusItem>,
}

impl GeniusApiBridge {
    /// Create a new API bridge with default settings
    pub fn new() -> Self {
        Self {
            api_client: GeniusApiClient::new(),
            last_response: None,
            request_in_progress: false,
            current_page: 1,
            current_query: String::new(),
            all_items: Vec::new(),
        }
    }

    /// Create a new API bridge with a custom API client
    pub fn with_client(api_client: GeniusApiClient) -> Self {
        Self {
            api_client,
            last_response: None,
            request_in_progress: false,
            current_page: 1,
            current_query: String::new(),
            all_items: Vec::new(),
        }
    }

    /// Configure the API client with the given API key and organization ID
    pub fn configure(&mut self, api_key: &str, organization_id: &str) {
        self.api_client = GeniusApiClient::new()
            .with_api_key(api_key.to_string())
            .with_organization_id(organization_id.to_string());
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
        // If the query text has changed, reset pagination
        if input != self.current_query {
            self.current_page = 1;
            self.current_query = input.to_string();
            self.all_items.clear();
        }
        
        self.execute_query_with_page(input, self.current_page)
    }

    /// Load the next page of results for the current query
    pub fn load_next_page(&mut self) -> Result<GeniusResponse, GeniusApiError> {
        println!("[DEBUG] GeniusApiBridge: load_next_page() called (current_page: {}, current_query: '{}')", 
            self.current_page, self.current_query);
            
        if self.current_query.is_empty() {
            println!("[DEBUG] GeniusApiBridge: load_next_page() failed - empty query");
            return Err(GeniusApiError::Other("No current query to load more results for".to_string()));
        }
        
        // Increment the page number
        self.current_page += 1;
        println!("[DEBUG] GeniusApiBridge: Incrementing page to {}", self.current_page);
        
        // Create local copies of the values we need
        let query = self.current_query.clone();
        let page = self.current_page;
        
        // Call execute_query_with_page with the local copies
        let result = self.execute_query_with_page(&query, page);
        
        // Log the result
        match &result {
            Ok(response) => {
                println!("[DEBUG] GeniusApiBridge: load_next_page() succeeded - got {} items", response.items.len());
            },
            Err(e) => {
                println!("[DEBUG] GeniusApiBridge: load_next_page() failed - {}", e);
            }
        }
        
        result
    }

    /// Execute a query with the given input string and page number
    /// 
    /// This is the core method that actually sends the query to the API
    /// and handles the response.
    fn execute_query_with_page(&mut self, query: &str, page: usize) -> Result<GeniusResponse, GeniusApiError> {
        // Mark that a request is in progress
        self.request_in_progress = true;
        
        println!("[DEBUG] GeniusApiBridge: Executing query: '{}' (page {})", query, page);
        
        // Execute the query using the API client with the specified page
        let result = self.api_client.query_sync_with_page(query, page);
        
        // Update the last response and request status
        match &result {
            Ok(response) => {
                println!("[DEBUG] GeniusApiBridge: Query successful, received {} items", response.items.len());
                
                // Store the response
                self.last_response = Some(response.clone());
                
                // If this is page 1, clear the all_items list
                if page == 1 {
                    self.all_items.clear();
                }
                
                // Add the new items to the all_items list
                self.all_items.extend(response.items.clone());
                
                self.request_in_progress = false;
            }
            Err(e) => {
                println!("[DEBUG] GeniusApiBridge: Query failed: {}", e);
                self.request_in_progress = false;
            }
        }
        
        result
    }

    /// Execute a query with the given input string (page 1)
    /// 
    /// This is a wrapper around execute_query_with_page for backward compatibility
    fn execute_query(&mut self, query: &str) -> Result<GeniusResponse, GeniusApiError> {
        self.execute_query_with_page(query, 1)
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

    /// Get all items loaded so far (across all pages)
    pub fn all_items(&self) -> &[GeniusItem] {
        &self.all_items
    }

    /// Get the current page number
    pub fn current_page(&self) -> usize {
        self.current_page
    }

    /// Check if there are more pages to load
    /// 
    /// For now, we'll assume there are always more pages to load
    /// In a real implementation, this would check if we've reached the end of the results
    pub fn has_more_pages(&self) -> bool {
        // In a real implementation, this would check if we've reached the end of the results
        // For now, we'll assume there are always more pages to load if we have a current query
        let has_more = !self.current_query.is_empty();
        println!("[DEBUG] GeniusApiBridge: has_more_pages() = {} (current_query: '{}', current_page: {})", 
            has_more, self.current_query, self.current_page);
        has_more
    }
}

/// Factory functions for creating API bridges
pub mod factory {
    use super::*;
    use std::env;

    /// Create a default API bridge
    pub fn create_default_bridge() -> GeniusApiBridge {
        GeniusApiBridge::new()
    }

    /// Create a mock API bridge for testing
    pub fn create_mock_bridge() -> GeniusApiBridge {
        let mock_client = super::super::genius_api::mock::create_mock_client();
        GeniusApiBridge::with_client(mock_client)
    }

    /// Create a configured API bridge with the given API key and organization ID
    pub fn create_configured_bridge(api_key: &str, organization_id: &str) -> GeniusApiBridge {
        let mut bridge = GeniusApiBridge::new();
        bridge.configure(api_key, organization_id);
        bridge
    }
    
    /// Create an API bridge configured from environment variables
    /// 
    /// This function looks for GENIUS_API_KEY and GENIUS_ORGANIZATION_ID
    /// environment variables and uses them to configure the API bridge.
    /// 
    /// # Returns
    /// 
    /// Returns a configured GeniusApiBridge if both environment variables
    /// are found, otherwise returns a default bridge that will use mock data.
    pub fn create_from_env() -> GeniusApiBridge {
        let api_key = env::var("GENIUS_API_KEY").ok();
        let org_id = env::var("GENIUS_ORGANIZATION_ID").ok();
        
        match (api_key, org_id) {
            (Some(key), Some(org)) if !key.is_empty() && !org.is_empty() => {
                println!("[INFO] Configuring Genius API with environment variables");
                create_configured_bridge(&key, &org)
            },
            _ => {
                println!("[WARN] Missing environment variables for Genius API");
                println!("[WARN] Set GENIUS_API_KEY and GENIUS_ORGANIZATION_ID to use the real API");
                println!("[WARN] Falling back to mock data");
                create_default_bridge()
            }
        }
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
            
            // Check that the items have the expected content
            for (i, item) in response.items.iter().enumerate() {
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
