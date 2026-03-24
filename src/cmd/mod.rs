use std::cell::RefCell;

use crate::{
    cmd::ext::CommandRunner,
    errors::ShellErrors,
    expand::{self, expand_commands},
    parser::{Parser, lex_words, parse_words},
    state::ShellState,
};

pub mod ext;

pub enum BuiltIn {
    Cd(String),
    Export(String, String),
    Exit,
}

pub fn is_builtin(cmd: &str) -> bool {
    matches!(
        cmd,
        "cd" | "export" | "exit"
    )
}

fn parse_builtin_cmd(cmd: &str) -> Option<BuiltIn> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    match parts[0] {
        "cd" => Some(BuiltIn::Cd(parts[1].to_string())),
        "export" => Some(BuiltIn::Export(parts[1].to_string(), parts[2].to_string())),
        "exit" => Some(BuiltIn::Exit),
        _ => None,
    }
}

pub fn run_cmd(cmd: &str, state: &mut ShellState) -> Result<(), ShellErrors> {
    if let Some(builtin) = parse_builtin_cmd(cmd) {
        return run_builtin_cmd(builtin, state);
    }

    let words = lex_words(cmd);
    let tokens = parse_words(words);

    println!("Tokens: {:?}", tokens);

    let mut parser = Parser::new(tokens);
    let cmds = parser.parse_tokens()?;
    println!("Parsed Commands: {:?}", cmds);

    let expanded_cmds = expand_commands(cmds, &state);
    println!("Expanded Commands: {:?}", expanded_cmds);

    let mut runner = CommandRunner::new(RefCell::new(state));
    println!("Running Commands...");
    runner.run_ext_cmds(expanded_cmds)
}

pub fn run_builtin_cmd(cmd: BuiltIn, state: &mut ShellState) -> Result<(), ShellErrors> {
    match cmd {
        BuiltIn::Cd(path) => state.change_dir("cd", &path),
        BuiltIn::Export(key, value) => {
            state.set_env_var(key, value);
            Ok(())
        }
        BuiltIn::Exit => {
            println!("Exiting shell...");
            std::process::exit(0);
        }
    }
}
