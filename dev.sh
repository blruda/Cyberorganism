#!/bin/bash
# Development workflow script for Cyberorganism

set -e  # Exit on any error

# Parse command line arguments
NO_INSTALL=false
NO_SERVER=true  # Default to not starting server unless --start-server flag is provided

for arg in "$@"; do
  case $arg in
    --no-install)
      NO_INSTALL=true
      ;;
    --start-server)
      NO_SERVER=false
      ;;
  esac
done

echo "=== Cyberorganism Development Workflow ==="
echo "This script will build both the plugin and the server."

# Build the plugin first
echo -e "\n=== Building Logseq Plugin ==="
./build-plugin.sh

# Build the Rust server
echo -e "\n=== Building Rust Server ==="
./server.sh build

# Install the plugin (can be skipped with --no-install flag)
echo -e "\n=== Installation ==="
if [[ "$NO_INSTALL" == "true" ]]; then
  echo "Skipping plugin installation (--no-install flag provided)"
else
  echo "Installing plugin to Logseq..."
  mkdir -p ~/.logseq/plugins/logseq-cyberorganism
  cp -r src/plugin/dist/* ~/.logseq/plugins/logseq-cyberorganism/
  cp src/plugin/package.json ~/.logseq/plugins/logseq-cyberorganism/
  echo "Plugin installed to ~/.logseq/plugins/logseq-cyberorganism/"
  echo "Please restart Logseq or reload the plugin to see the changes."
fi

# Start the server if --start-server flag is provided
if [[ "$NO_SERVER" == "false" ]]; then
  echo -e "\n=== Starting Server ==="
  echo "Starting server (any existing server will be stopped)..."
  ./server.sh restart
else
  echo -e "\n=== Build Complete ==="
  echo "To start the server: ./server.sh start"
  echo "To stop the server: ./server.sh stop"
  echo "To restart the server: ./server.sh restart"
  echo -e "\nHappy coding!"
fi
