//! Debug logging functionality for development purposes.
//! This module provides utilities for logging debug information to a file
//! during development and testing.

use chrono::Local;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Once;

static DEBUG_INIT: Once = Once::new();
static mut DEBUG_FILE: Option<File> = None;

/// Initializes the debug log file.
/// This is called automatically the first time `log_debug` is used.
fn init_debug_log() {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("debug.log")
        .unwrap_or_else(|e| {
            eprintln!("Failed to initialize debug log: {e}");
            std::process::exit(1);
        });

    unsafe {
        DEBUG_FILE = Some(file);
    }
}

/// Logs a debug message to the debug log file
pub fn log_debug(msg: &str) {
    DEBUG_INIT.call_once(init_debug_log);

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");

    unsafe {
        let debug_file = &raw mut DEBUG_FILE;
        if let Some(file) = (*debug_file).as_mut() {
            if writeln!(file, "[{timestamp}] {msg}").is_err() {
                eprintln!("Failed to write to debug log");
            }
        }
    }
}
