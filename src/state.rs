use std::{collections::HashMap, path::PathBuf};

use crate::errors::ShellErrors;


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

    pub fn set_env_var(&mut self, key: String, value: String) {
        self.env_vars.insert(key, value);
    }

    pub fn unset_env_var(&mut self, key: &str) {
        self.env_vars.remove(key);
    }

    pub fn get_env_var(&self, key: &str) -> Option<&String> {
        self.env_vars.get(key)
    }

    pub fn append_history(&mut self, cmd: String) {
        self.cmd_history.push(cmd);
    }

    // change current directory
    // 1. if the path is absolute, change to that path
    // 2. if the path is relative, change to that path relative to current directory
    pub fn change_dir(&mut self, cmd: &str, path: &str) -> Result<(), ShellErrors> {
        let new_path = if PathBuf::from(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.current_dir.join(path)
        };

        // check the new path exists and is a directory
        if new_path.exists() && new_path.is_dir() {
            self.current_dir = new_path;
            return Ok(());
        }

        Err(ShellErrors::CmdError(cmd.to_string(), format!("No such directory: {}", path)))
    }
}