use std::{collections::HashMap, path::PathBuf};


// current shell status, such as current directory, environment variables, etc.
pub struct ShellState {
    pub current_dir: PathBuf,
    pub env_vars: HashMap<String, String>,
    pub cmd_history: Vec<String>,
    pub exit_code: i32,
}

impl ShellState {
    pub fn new() -> Self {
        Self {
            current_dir: std::env::current_dir().unwrap(),
            env_vars: std::env::vars().collect(),
            cmd_history: Vec::new(),
            exit_code: 0,
        }
    }
}