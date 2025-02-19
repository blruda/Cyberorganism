use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

// Task struct represents a single task in our system
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: u32,                   // Unique identifier for the task
    pub content: String,           // The actual task content
    pub created_at: DateTime<Utc>, // When the task was created
    pub container: TaskContainer,  // Current container of the task
    pub status: TaskStatus,        // Current status of the task
}

// TaskContainer represents where the task is located in our system
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TaskContainer {
    Taskpad,
    Backburner,
    Shelved,
    Archived,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TaskStatus {
    Todo,
    Doing,
    Done,
}

// Functions for task persistence
pub fn save_tasks(tasks: &[Task]) -> io::Result<()> {
    let json = serde_json::to_string_pretty(&tasks)?;
    fs::write("tasks.json", json)?;
    Ok(())
}

pub fn load_tasks() -> io::Result<Vec<Task>> {
    if Path::new("tasks.json").exists() {
        let json = fs::read_to_string("tasks.json")?;
        Ok(serde_json::from_str(&json)?)
    } else {
        Ok(Vec::new())
    }
}
