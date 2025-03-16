//! Configuration management for the application.
//! 
//! This module provides functionality for loading and accessing configuration
//! settings from various sources, including config files and environment variables.
//! 
#![allow(dead_code)]

use config::{Config, ConfigError, File};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;

/// Application configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Configuration for the Genius API
    #[serde(default)]
    pub genius: GeniusConfig,
}

/// Configuration for the Genius API
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeniusConfig {
    /// API key for the Genius Platform
    #[serde(default)]
    pub api_key: Option<String>,
    
    /// Base URL for the Genius API
    #[serde(default = "default_genius_api_url")]
    pub base_url: String,
    
    /// Timeout in seconds for API requests
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
}

/// Default base URL for the Genius API
fn default_genius_api_url() -> String {
    "https://api.genius.example.com".to_string()
}

/// Default timeout in seconds
fn default_timeout_secs() -> u64 {
    10
}

// Global configuration instance
static CONFIG: OnceLock<AppConfig> = OnceLock::new();

/// Get the application configuration
/// 
/// This function returns a reference to the global configuration instance.
/// If the configuration hasn't been loaded yet, it will attempt to load it.
pub fn get_config() -> &'static AppConfig {
    CONFIG.get_or_init(|| {
        // Try to load the configuration, or use defaults if loading fails
        match load_config() {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Failed to load configuration: {}", e);
                AppConfig {
                    genius: GeniusConfig::default(),
                }
            }
        }
    })
}

/// Load the configuration from various sources
/// 
/// This function loads configuration from the following sources, in order:
/// 1. Default values
/// 2. Configuration file in the system config directory
/// 3. Configuration file in the current directory
/// 4. Environment variables (prefixed with "CYBERORGANISM_")
fn load_config() -> Result<AppConfig, ConfigError> {
    let mut builder = Config::builder()
        // Start with default values
        .set_default("genius.base_url", default_genius_api_url())?
        .set_default("genius.timeout_secs", default_timeout_secs())?;
    
    // Try to load from system config directory
    if let Some(config_path) = get_system_config_path() {
        builder = builder.add_source(File::from(config_path).required(false));
    }
    
    // Try to load from current directory
    builder = builder.add_source(File::with_name("config").required(false));
    
    // Add environment variables (prefixed with "CYBERORGANISM_")
    builder = builder.add_source(config::Environment::with_prefix("CYBERORGANISM").separator("_"));
    
    // Build the configuration
    let config = builder.build()?;
    
    // Deserialize the configuration
    let app_config: AppConfig = config.try_deserialize()?;
    
    Ok(app_config)
}

/// Get the path to the system configuration file
fn get_system_config_path() -> Option<PathBuf> {
    // Get the project directories
    let proj_dirs = ProjectDirs::from("com", "cyberorganism", "cyberorganism")?;
    
    // Create the config directory if it doesn't exist
    let config_dir = proj_dirs.config_dir();
    std::fs::create_dir_all(config_dir).ok()?;
    
    // Return the path to the config file
    Some(config_dir.join("config.toml"))
}

/// Initialize the configuration system
/// 
/// This function should be called early in the application startup process
/// to ensure that the configuration is loaded before it's needed.
pub fn init() {
    // Force the configuration to be loaded
    let _ = get_config();
}
