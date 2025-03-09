#!/bin/bash
# Server management script for Cyberorganism

set -e  # Exit on any error

# Function to find and kill existing server processes
stop_server() {
  echo "Checking for running server instances..."
  SERVER_PID=$(ps aux | grep "target/debug/server" | grep -v grep | awk '{print $2}')
  
  if [ -n "$SERVER_PID" ]; then
    echo "Found running server with PID: $SERVER_PID, stopping it..."
    kill $SERVER_PID
    sleep 1  # Give it a moment to shut down
    echo "Server stopped."
  else
    echo "No running server found."
  fi
}

# Function to build and start the server
start_server() {
  echo "Building server..."
  cargo build --bin server
  
  echo "Starting server..."
  cargo run --bin server
}

# Function to restart the server
restart_server() {
  stop_server
  start_server
}

# Main script logic
case "$1" in
  start)
    stop_server  # Stop any existing server first to avoid port conflicts
    start_server
    ;;
  stop)
    stop_server
    ;;
  restart)
    restart_server
    ;;
  build)
    cargo build --bin server
    echo "Server built successfully."
    ;;
  *)
    echo "Usage: $0 {start|stop|restart|build}"
    echo ""
    echo "Commands:"
    echo "  start    - Build and start the server (stopping any existing instances)"
    echo "  stop     - Stop any running server instances"
    echo "  restart  - Stop and restart the server"
    echo "  build    - Build the server without starting it"
    exit 1
    ;;
esac

exit 0
