#!/bin/bash

# Build the plugin
./build-plugin.sh

# Copy the plugin files to Logseq plugins directory
PLUGIN_DIR=~/.logseq/plugins/logseq-cyberorganism
echo "Installing plugin to $PLUGIN_DIR"

# Create the directory if it doesn't exist
mkdir -p "$PLUGIN_DIR"

# Copy the plugin files
cp -r src/plugin/dist/* "$PLUGIN_DIR"

echo "Plugin installed successfully!"
echo "Please restart Logseq or reload the plugin to see the changes."
