#!/bin/bash
# Build script for the Cyberorganism Logseq plugin

set -e  # Exit on any error

echo "Building Cyberorganism Logseq plugin..."

# Navigate to plugin directory
cd "$(dirname "$0")/src/plugin"

# Install dependencies if node_modules doesn't exist
if [ ! -d "node_modules" ]; then
  echo "Installing dependencies..."
  npm install
fi

# Create dist directory if it doesn't exist
mkdir -p dist

# Build with webpack
echo "Building with webpack..."
npx webpack --mode=production

# Copy static files
echo "Copying static files..."
cp index.html manifest.json icon.svg icon.png dist/

echo "Build complete! Plugin files are in src/plugin/dist/"
echo "Remember to enable Developer Mode in Logseq and load the plugin from this directory."
