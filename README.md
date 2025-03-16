# Cyberorganism

Cyberorganism is a keyboard-driven Personal Knowledge Management (PKM) application built in Rust. It provides a minimalist interface for managing tasks and notes with a focus on efficiency and intelligent context management.

## Features

- **Keyboard-First Interface**: Navigate and manage your tasks without touching the mouse
- **Task Management**: Create, organize, and track tasks in a hierarchical outliner
- **Minimalist UI**: Clean, distraction-free interface built with egui
- **Genius Platform Integration**: Access intelligent content suggestions through the Genius API

## Status

This is an early prototype focused on core functionality. Future development plans include AI integration for enhanced knowledge management capabilities.

## Building

```
cargo build
```

## Running

```
cargo run
```

## Genius API Configuration

The application integrates with the Genius Platform API. By default, it will use mock data if no API key is provided.

### Setting the API Key

There are several ways to provide the API key:

1. **Using environment variables** (current method):
   
   The application is currently configured to use environment variables for API authentication:
   
   ```bash
   # Set these environment variables with your actual credentials
   export GENIUS_API_KEY="your-api-key-here"
   export GENIUS_ORGANIZATION_ID="your-organization-id-here"
   ```
   
   Alternatively, you can create a `.env` file in the project root:
   
   ```
   # Genius API Configuration
   GENIUS_API_KEY=your-api-key-here
   GENIUS_ORGANIZATION_ID=your-organization-id-here
   ```
   
   The application will automatically load these variables when it starts.

2. **Using config.toml** (in progress):
   
   Support for configuration via `config.toml` is currently in development and will be the recommended method in future releases.
   
   ```bash
   # First, copy the example configuration file
   cp config.toml.example config.toml
   
   # Then edit config.toml and add your API key and organization ID
   ```
   
   Example `config.toml` structure (not yet implemented):
   ```toml
   [genius]
   api_key = "your-api-key-here"
   organization_id = "your-organization-id-here"
   ```

3. **System-wide configuration** (planned):
   ```bash
   mkdir -p ~/.config/cyberorganism/
   echo '[genius]\napi_key = "your-api-key-here"\norganization_id = "your-organization-id-here"' > ~/.config/cyberorganism/config.toml
   ```

### Feature Flags

The application uses feature flags to control API behavior:

- **Default mode** (no flags): Uses real API if an API key is provided, falls back to mock data if not
  ```bash
  cargo run
  ```

- **Force mock mode** (even if API key is provided):
  ```bash
  cargo run --features mock-api
  ```

- **Explicit real API mode** (same as default, for clarity):
  ```bash
  cargo run --no-default-features
  ```

## Genius API Documentation

> **Note**: This section is a placeholder. Detailed API documentation will be added once the final API schema is available.

### Overview

The Genius Platform API provides intelligent content suggestions based on user input. The API can be queried with text and returns relevant suggestions.

### Endpoints

- **Query Endpoint**: Used to retrieve suggestions based on user input
  - URL: `https://api.genius.example.com/query`
  - Method: `POST`
  - Authentication: Bearer token

### Request/Response Format

Placeholder for the request and response format documentation. This will be updated with the final schema when available.

## License

MIT License