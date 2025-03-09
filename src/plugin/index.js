import '@logseq/libs';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import 'xterm/css/xterm.css';

// Debug helper function to log to console only
function debugLog(message) {
  const timestamp = new Date().toISOString();
  const logMessage = `[${timestamp}] ${message}`;
  console.log(logMessage);
  
  // UI notifications are disabled to avoid clutter
  // Uncomment the following lines to enable UI notifications for debugging
  // if (typeof logseq !== 'undefined') {
  //   logseq.UI.showMsg(message, 'info', { timeout: 3000 });
  // }
}

// Backend server URL
const BACKEND_URL = 'http://127.0.0.1:3030';

// Initialize the terminal
let terminal;
let fitAddon;
let terminalContainer;
let terminalSidebar;
let websocket;

/**
 * Main plugin initialization
 */
async function main() {
  debugLog('Cyberorganism plugin loaded');

  // Register UI elements
  logseq.provideStyle(`
    .terminal-sidebar {
      width: 300px;
      height: 100%;
      position: fixed;
      right: 0;
      top: 0;
      background-color: var(--ls-primary-background-color);
      border-left: 1px solid var(--ls-border-color);
      z-index: 999;
      display: flex;
      flex-direction: column;
      overflow: hidden;
    }
  `);

  // Register slash command to open terminal
  logseq.Editor.registerSlashCommand('terminal', async () => {
    openTerminal();
  });

  // Register keyboard shortcut to toggle terminal
  logseq.App.registerCommandPalette({
    key: 'toggle-terminal',
    label: 'Toggle Terminal',
    keybinding: {
      binding: 'ctrl+`',
    },
    callback: () => {
      toggleTerminal();
    },
  });

  // Add icon to toolbar
  logseq.App.registerUIItem('toolbar', {
    key: 'terminal-button',
    template: `
      <a class="button" data-on-click="toggleTerminal">
        <i class="ti ti-terminal"></i>
      </a>
    `,
  });
  debugLog('Terminal button registered in toolbar');
  
  // Add a direct event listener to test button functionality
  setTimeout(() => {
    try {
      // Try multiple selectors to find the button
      const terminalButton = document.querySelector('a[data-on-click="toggleTerminal"]') || 
                            document.querySelector('.toolbar-terminal-button') ||
                            document.querySelector('a.button i.ti-terminal')?.parentElement;
      
      if (terminalButton) {
        debugLog('Found terminal button in DOM, adding direct click listener');
        // Force a UI notification to confirm the button was found
        logseq.UI.showMsg('Terminal button found in DOM', 'info', { timeout: 2000 });
        
        // Add click listener with capture to ensure it's triggered
        terminalButton.addEventListener('click', (e) => {
          debugLog('Terminal button clicked directly');
          logseq.UI.showMsg('Terminal button clicked directly', 'info', { timeout: 2000 });
          toggleTerminal();
        }, true);
      } else {
        debugLog('Could not find terminal button in DOM');
        logseq.UI.showMsg('Could not find terminal button in DOM', 'warning', { timeout: 3000 });
      }
    } catch (error) {
      debugLog(`Error setting up direct click handler: ${error.message}`);
      logseq.UI.showMsg(`Error setting up click handler: ${error.message}`, 'error', { timeout: 3000 });
    }
  }, 3000); // Wait longer for DOM to be ready

  // Register model for handling UI interactions
  logseq.provideModel({
    toggleTerminal() {
      debugLog('toggleTerminal called from UI interaction');
      try {
        toggleTerminal();
      } catch (error) {
        debugLog(`Error in toggleTerminal: ${error.message}`);
        console.error('Error in toggleTerminal:', error);
      }
    },
  });

  debugLog('Cyberorganism plugin initialized');
}

/**
 * Create and open the terminal sidebar
 */
function openTerminal() {
  // Create sidebar if it doesn't exist
  if (!terminalSidebar) {
    terminalSidebar = document.createElement('div');
    terminalSidebar.classList.add('terminal-sidebar');
    document.body.appendChild(terminalSidebar);

    // Create terminal container
    terminalContainer = document.createElement('div');
    terminalContainer.id = 'terminal-container';
    terminalContainer.style.width = '100%';
    terminalContainer.style.height = '100%';
    terminalSidebar.appendChild(terminalContainer);

    // Initialize terminal
    terminal = new Terminal({
      cursorBlink: true,
      fontFamily: 'Menlo, DejaVu Sans Mono, Consolas, monospace',
      fontSize: 14,
      theme: {
        background: '#1e1e1e',
        foreground: '#f0f0f0',
      },
    });

    // Add fit addon to make terminal resize to container
    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);

    // Open terminal in container
    terminal.open(terminalContainer);
    fitAddon.fit();

    // Connect to backend
    connectToBackend();

    // Handle terminal input
    terminal.onData((data) => {
      if (websocket && websocket.readyState === WebSocket.OPEN) {
        websocket.send(JSON.stringify({ type: 'input', data }));
      }
    });

    // Handle window resize
    window.addEventListener('resize', () => {
      if (fitAddon) {
        fitAddon.fit();
        if (websocket && websocket.readyState === WebSocket.OPEN) {
          const { cols, rows } = terminal;
          websocket.send(JSON.stringify({ type: 'resize', cols, rows }));
        }
      }
    });
  }

  // Show terminal sidebar
  terminalSidebar.style.display = 'flex';
  
  // Fit terminal to container
  setTimeout(() => {
    if (fitAddon) {
      fitAddon.fit();
      if (websocket && websocket.readyState === WebSocket.OPEN) {
        const { cols, rows } = terminal;
        websocket.send(JSON.stringify({ type: 'resize', cols, rows }));
      }
    }
  }, 100);
}

/**
 * Toggle terminal sidebar visibility
 */
function toggleTerminal() {
  debugLog('toggleTerminal function called');
  // Show a notification for button clicks to confirm it's working
  if (typeof logseq !== 'undefined') {
    logseq.UI.showMsg('Terminal button clicked', 'info', { timeout: 2000 });
  }
  try {
    if (!terminalSidebar) {
      debugLog('Terminal sidebar does not exist, creating it');
      openTerminal();
      return;
    }

  if (terminalSidebar.style.display === 'none') {
    debugLog('Showing terminal sidebar');
    terminalSidebar.style.display = 'flex';
    setTimeout(() => {
      if (fitAddon) {
        fitAddon.fit();
        debugLog('Fit terminal to container');
      }
    }, 100);
  } else {
    debugLog('Hiding terminal sidebar');
    terminalSidebar.style.display = 'none';
  }
  } catch (error) {
    debugLog(`Error in toggleTerminal function: ${error.message}`);
    console.error('Error in toggleTerminal function:', error);
  }
}

/**
 * Connect to the Rust backend server
 */
function connectToBackend() {
  debugLog(`Connecting to backend server at ${BACKEND_URL}`);
  // Check if backend is running
  fetch(`${BACKEND_URL}/health`)
    .then(response => {
      debugLog(`Health check response: ${response.status}`);
      if (response.ok) {
        // Backend is running, connect via WebSocket
        debugLog('Health check successful, connecting to WebSocket');
        connectWebSocket();
      } else {
        debugLog(`Health check failed with status: ${response.status}`);
        // Backend is not running, show error
        terminal.writeln('\r\n\x1b[31mBackend server is not running.\x1b[0m');
        terminal.writeln('\x1b[33mPlease start the backend server with:\x1b[0m');
        terminal.writeln('\x1b[1;34mcd /home/brandt/projects/cyberorganism && cargo run --bin server\x1b[0m\r\n');
      }
    })
    .catch(error => {
      console.error('Error connecting to backend:', error);
      debugLog(`Connection error details: ${error.message}`);
      terminal.writeln('\r\n\x1b[31mCannot connect to backend server.\x1b[0m');
      terminal.writeln('\x1b[33mPlease start the backend server with:\x1b[0m');
      terminal.writeln('\x1b[1;34mcd /home/brandt/projects/cyberorganism && cargo run --bin server\x1b[0m\r\n');
    });
}

/**
 * Connect to the backend via WebSocket
 */
function connectWebSocket() {
  debugLog('Connecting to WebSocket at ws://127.0.0.1:3030/terminal');
  websocket = new WebSocket(`ws://127.0.0.1:3030/terminal`);

  websocket.onopen = () => {
    debugLog('WebSocket connected');
    // Send terminal size on connection
    const { cols, rows } = terminal;
    websocket.send(JSON.stringify({ type: 'resize', cols, rows }));
    
    // Clear terminal and show welcome message
    terminal.clear();
    terminal.writeln('\r\n\x1b[1;32mCyberorganism Terminal\x1b[0m');
    terminal.writeln('\x1b[90mConnected to backend server\x1b[0m\r\n');
  };

  websocket.onmessage = (event) => {
    try {
      const message = JSON.parse(event.data);
      
      if (message.type === 'output') {
        terminal.write(message.data);
      } else if (message.type === 'error') {
        terminal.writeln(`\r\n\x1b[31mError: ${message.data}\x1b[0m\r\n`);
      }
    } catch (error) {
      console.error('Error parsing WebSocket message:', error);
      terminal.write(event.data);
    }
  };

  websocket.onclose = () => {
    debugLog('WebSocket disconnected');
    terminal.writeln('\r\n\x1b[31mDisconnected from backend server.\x1b[0m\r\n');
    
    // Try to reconnect after a delay
    setTimeout(() => {
      if (terminalSidebar && terminalSidebar.style.display !== 'none') {
        connectToBackend();
      }
    }, 5000);
  };

  websocket.onerror = (error) => {
    console.error('WebSocket error:', error);
    terminal.writeln('\r\n\x1b[31mWebSocket error. Reconnecting...\x1b[0m\r\n');
  };
}

// Initialize the plugin
logseq.ready(main).catch(console.error);
