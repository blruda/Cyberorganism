use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::get, Router,
};
use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
    thread,
};
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};
use log::{info, debug, error, warn};
use futures::{StreamExt, SinkExt};

// Terminal session state
struct TerminalSession {
    pty_pair: PtyPair,
    reader_thread: Option<thread::JoinHandle<()>>,
    #[allow(dead_code)]
    writer_thread: Option<thread::JoinHandle<()>>,
}

// Application state
struct AppState {
    terminal_sessions: Mutex<Vec<Arc<Mutex<TerminalSession>>>>,
}

// WebSocket message types
#[derive(Serialize, Deserialize, Debug)]
struct TerminalMessage {
    #[serde(rename = "type")]
    message_type: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cols: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rows: Option<u16>,
}

#[tokio::main]
async fn main() {
    // Initialize logger with debug level
    unsafe {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();
    
    info!("Starting terminal server");
    
    // Create application state
    let app_state = Arc::new(AppState {
        terminal_sessions: Mutex::new(Vec::new()),
    });
    
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    // Create router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/terminal", get(terminal_handler))
        .layer(cors)
        .with_state(app_state);
        
    info!("Router configured with health and terminal endpoints");
    
    // Start server
    let addr = "127.0.0.1:3030";
    info!("Starting server on {}", addr);
    info!("Server will be accessible at http://{}/health and ws://{}/terminal", addr, addr);
    
    // Try to bind to the address
    match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            info!("Server started successfully on {}", addr);
            axum::serve(listener, app).await.unwrap();
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AddrInUse {
                // Address is already in use, try to kill the existing process
                info!("Address {} already in use. Attempting to stop existing server...", addr);
                
                // On Linux, we can use the fuser command to find and kill processes using the port
                if let Ok(output) = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(format!("lsof -ti tcp:3030 | xargs kill -9"))
                    .output() 
                {
                    if output.status.success() {
                        info!("Successfully terminated existing server process");
                        
                        // Wait a moment for the port to be released
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        
                        // Try binding again
                        match tokio::net::TcpListener::bind(addr).await {
                            Ok(listener) => {
                                info!("Server restarted successfully on {}", addr);
                                axum::serve(listener, app).await.unwrap();
                            }
                            Err(e) => {
                                eprintln!("Failed to bind to {} after killing existing process: {}", addr, e);
                                std::process::exit(1);
                            }
                        }
                    } else {
                        eprintln!("Failed to terminate existing server process");
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                } else {
                    eprintln!("Failed to execute command to find existing process");
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            } else {
                eprintln!("Failed to bind to {}: {}", addr, e);
                std::process::exit(1);
            }
        }
    };
}

// Health check endpoint
async fn health_check(headers: axum::http::HeaderMap) -> impl IntoResponse {
    info!("Health check requested with headers: {:?}", headers);
    StatusCode::OK
}

// Terminal WebSocket handler
async fn terminal_handler(
    ws: WebSocketUpgrade,
    headers: axum::http::HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    info!("Terminal WebSocket connection requested with headers: {:?}", headers);
    info!("User-Agent: {:?}", headers.get("user-agent"));
    info!("Origin: {:?}", headers.get("origin"));
    ws.on_upgrade(|socket| handle_terminal_socket(socket, state))
}

// Handle terminal WebSocket connection
async fn handle_terminal_socket(socket: WebSocket, state: Arc<AppState>) {
    info!("Terminal WebSocket connection established");
    // Split the socket
    let (mut sender, mut receiver) = socket.split();
    
    // Create channels for communication between tasks
    let (tx, mut rx) = mpsc::channel::<Message>(100);
    
    // Create a new PTY
    let pty_system = native_pty_system();
    let pty_pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("Failed to open PTY");
    
    // Start a shell in the PTY
    let mut cmd = CommandBuilder::new("bash");
    cmd.env("TERM", "xterm-256color");
    
    let mut child = pty_pair
        .slave
        .spawn_command(cmd)
        .expect("Failed to spawn command");
    
    // Create a new terminal session
    let session = Arc::new(Mutex::new(TerminalSession {
        pty_pair,
        reader_thread: None,
        writer_thread: None,
    }));
    
    // Add the session to the application state
    {
        let mut sessions = state.terminal_sessions.lock().unwrap();
        sessions.push(session.clone());
    }
    
    // Clone the session for the reader thread
    let reader_session = session.clone();
    
    // Start a task to read from the PTY and send to the WebSocket
    let reader_thread = thread::spawn(move || {
        let tx = tx.clone();
        let session = reader_session.lock().unwrap();
        let mut reader = session.pty_pair.master.try_clone_reader().unwrap();
        let mut buffer = [0u8; 1024];
        
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buffer[..n]).to_string();
                    let message = serde_json::to_string(&TerminalMessage {
                        message_type: "output".to_string(),
                        data: Some(data),
                        cols: None,
                        rows: None,
                    })
                    .unwrap();
                    
                    if let Err(_) = tx.blocking_send(Message::Text(message)) {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    
    // Update the session with the reader thread
    {
        let mut session = session.lock().unwrap();
        session.reader_thread = Some(reader_thread);
    }
    
    // Clone the session for the writer task
    let writer_session = session.clone();
    
    // Start a task to forward messages from the WebSocket to the PTY
    let writer_task = tokio::spawn(async move {
        info!("WebSocket reader task started");
        while let Some(Ok(msg)) = receiver.next().await {
            debug!("Received WebSocket message");
            if let Message::Text(text) = msg {
                debug!("Received text message: {}", text);
                if let Ok(terminal_msg) = serde_json::from_str::<TerminalMessage>(&text) {
                    debug!("Parsed terminal message: type={}", terminal_msg.message_type);
                    let session = writer_session.lock().unwrap();
                    
                    match terminal_msg.message_type.as_str() {
                        "input" => {
                            debug!("Processing input message");
                            if let Ok(mut writer) = session.pty_pair.master.take_writer() {
                                if let Some(data) = &terminal_msg.data {
                                    match writer.write_all(data.as_bytes()) {
                                    Ok(_) => debug!("Successfully wrote to PTY"),
                                    Err(e) => error!("Failed to write to PTY: {}", e)
                                    }
                                } else {
                                    warn!("Input message missing data field");
                                }
                            } else {
                                error!("Failed to get PTY writer");
                            }
                        }
                        "resize" => {
                            if let (Some(cols), Some(rows)) = (terminal_msg.cols, terminal_msg.rows) {
                                debug!("Resizing terminal to {}x{}", cols, rows);
                                match session.pty_pair.master.resize(PtySize {
                                    rows,
                                    cols,
                                    pixel_width: 0,
                                    pixel_height: 0,
                                }) {
                                    Ok(_) => debug!("Successfully resized PTY"),
                                    Err(e) => error!("Failed to resize PTY: {}", e)
                                }
                            } else {
                                warn!("Resize message missing cols or rows");
                            }
                        }
                        _ => {
                            warn!("Unknown message type: {}", terminal_msg.message_type);
                        }
                    }
                } else {
                    error!("Failed to parse terminal message: {}", text);
                }
            } else {
                debug!("Received non-text message");
            }
        }
        
        // When the WebSocket is closed, kill the child process
        let _ = child.kill();
    });
    
    // Start a task to forward messages from the PTY to the WebSocket
    let forward_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });
    
    // Wait for either task to complete
    tokio::select! {
        _ = writer_task => {},
        _ = forward_task => {},
    }
    
    // Clean up the session
    {
        let mut sessions = state.terminal_sessions.lock().unwrap();
        sessions.retain(|s| !Arc::ptr_eq(s, &session));
    }
}
