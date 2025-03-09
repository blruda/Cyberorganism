# Cyberorganism

A task management system with an integrated terminal, available as both a standalone TUI application and a Logseq plugin.

## Project Structure

- `src/server/`: Backend server for the Logseq plugin
- `src/plugin/`: Logseq plugin frontend
- `tui/`: Original TUI implementation (kept for reference)

## Running the Server (Default)

The server is the backend for the Logseq plugin and is the default target.

### Using the Server Management Script (Recommended)

```bash
# Start the server (stops any existing instances first)
./server.sh start

# Stop the server
./server.sh stop

# Restart the server
./server.sh restart

# Build the server without starting it
./server.sh build
```

This script automatically handles stopping any existing server instances before starting a new one, which prevents the "Address already in use" error.

### Manual Server Control

```bash
# Development build
cargo run --bin server

# Release build
cargo run --bin server --release
```

The server will start on http://localhost:3030.

## Running the TUI (Legacy)

The TUI version is kept for reference but is no longer the primary focus.

```bash
# Development build
cargo run --bin tui --features tui

# Release build
cargo run --bin tui --features tui --release
```

## Building and Installing the Logseq Plugin

### Building the Plugin

#### Using the Build Script (Recommended)

A build script is provided to automate the plugin build process:

```bash
# Make the script executable (first time only)
chmod +x build-plugin.sh

# Run the build script
./build-plugin.sh
```

This script will:
1. Install dependencies if needed
2. Build the plugin with webpack
3. Copy all necessary files to the dist directory

#### Manual Build

If you prefer to build manually:

```bash
# Navigate to the plugin directory
cd src/plugin

# Install dependencies
npm install

# Build the plugin
npx webpack --mode=production

# Copy necessary files to dist directory
cp index.html manifest.json icon.svg dist/
```

**Note**: You must rebuild the plugin every time you make changes to the plugin code.

### Installing in Logseq

#### Method 1: Manual Installation (Recommended)

1. Build the plugin using the build script:
   ```bash
   ./build-plugin.sh
   ```

2. Locate your Logseq plugins directory:
   - On Linux: `~/.logseq/plugins/`
   - On macOS: `~/Library/Application Support/logseq/plugins/`
   - On Windows: `%APPDATA%\Logseq\plugins\`

3. Create a directory for the plugin:
   ```bash
   mkdir -p ~/.logseq/plugins/logseq-cyberorganism
   ```

4. Copy the plugin files:
   ```bash
   cp -r src/plugin/dist/* ~/.logseq/plugins/logseq-cyberorganism/
   ```

5. Copy the package.json file (important for plugin recognition):
   ```bash
   cp src/plugin/package.json ~/.logseq/plugins/logseq-cyberorganism/
   ```

6. Restart Logseq

7. The plugin should appear in the toolbar with a terminal icon

#### Method 2: Using Developer Mode

1. Open Logseq
2. Enable Developer Mode:
   - Go to Settings (three dots in the top-right) > Advanced
   - Toggle on "Developer mode"
3. Restart Logseq
4. Go to Settings > Plugins
5. You should now see a "Load unpacked plugin" button
6. Click it and select the `src/plugin/dist` directory from this project

**Note**: If you encounter an "Illegal Logseq plugin package" error, use Method 1 instead.

### Using the Plugin

1. Start the backend server using the command above
2. Click the terminal icon in the Logseq toolbar
3. The terminal will appear in a sidebar
4. You can also use the `/terminal` slash command or the `Ctrl+\`` keyboard shortcut

## Development

### Prerequisites

- Rust (latest stable)
- Node.js and npm
- Logseq (for testing the plugin)

### Building Both Components

#### Using the Dev Script (Recommended)

A development script is provided to build both the server and plugin in one step:

```bash
# Make the script executable (first time only)
chmod +x dev.sh

# Run the development script
./dev.sh
```

This script will:
1. Build the Logseq plugin
2. Build the Rust server
3. Offer to install the plugin to Logseq
4. Offer to start the server

#### Using the Server Management Script

The `server.sh` script provides commands to manage the server:

```bash
# Make the script executable (first time only)
chmod +x server.sh

# Start the server (stops any existing instances first)
./server.sh start

# Stop the server
./server.sh stop

# Restart the server
./server.sh restart

# Build the server without starting it
./server.sh build
```

#### Manual Build

If you prefer to build manually:

```bash
# Build the server
cargo build --bin server

# Build the plugin
cd src/plugin && npm run build
```

## Troubleshooting

If the terminal doesn't connect to the backend:
1. Make sure the server is running (`cargo run --bin server`)
2. Check that the server is accessible at http://localhost:3030/health
3. If using a firewall, ensure port 3030 is open

## License

MIT
