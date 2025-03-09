// Simple test script to check server connectivity
const http = require('http');
const WebSocket = require('ws');

const BACKEND_URL = 'http://127.0.0.1:3030';

// Test health endpoint
console.log(`Testing connection to ${BACKEND_URL}/health...`);
http.get(`${BACKEND_URL}/health`, (res) => {
  console.log(`Health check response status: ${res.statusCode}`);
  
  if (res.statusCode === 200) {
    console.log('Health check successful!');
    testWebSocket();
  } else {
    console.error(`Health check failed with status: ${res.statusCode}`);
  }
}).on('error', (error) => {
  console.error(`Error connecting to backend: ${error.message}`);
});

// Test WebSocket connection
function testWebSocket() {
  console.log(`Testing WebSocket connection to ws://127.0.0.1:3030/terminal...`);
  const ws = new WebSocket('ws://127.0.0.1:3030/terminal');
  
  ws.onopen = () => {
    console.log('WebSocket connection successful!');
    // Send a test resize message
    ws.send(JSON.stringify({ type: 'resize', cols: 80, rows: 24 }));
    
    // Close after 2 seconds
    setTimeout(() => {
      console.log('Closing WebSocket connection...');
      ws.close();
    }, 2000);
  };
  
  ws.onmessage = (event) => {
    console.log(`Received WebSocket message: ${event.data}`);
  };
  
  ws.onerror = (error) => {
    console.error(`WebSocket error: ${error.message || 'Unknown error'}`);
  };
  
  ws.onclose = () => {
    console.log('WebSocket connection closed');
  };
}
