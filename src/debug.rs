//! Debug logging functionality for development purposes.
//! This module provides utilities for logging debug information to a file
//! during development and testing.

use chrono::Local;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::{Mutex, Once};

static DEBUG_INIT: Once = Once::new();
static DEBUG_FILE: Mutex<Option<File>> = Mutex::new(None);

/// Initializes the debug log file.
/// This is called automatically the first time `log_debug` is used.
fn init_debug_log() {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("debug.log")
        .unwrap_or_else(|e| {
            eprintln!("Failed to initialize debug log: {e}");
            std::process::exit(1);
        });

    *DEBUG_FILE.lock().unwrap() = Some(file);
}

/// Logs a debug message to the debug log file
pub fn log_debug(msg: &str) {
    DEBUG_INIT.call_once(init_debug_log);

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");

    if let Ok(mut guard) = DEBUG_FILE.lock() {
        if let Some(file) = guard.as_mut() {
            if writeln!(file, "[{timestamp}] {msg}").is_err() {
                eprintln!("Failed to write to debug log");
            }
        }
    }
}
