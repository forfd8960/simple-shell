use crate::{errors::ShellErrors, state::ShellState};

pub enum BuiltIn {
    Cd(String),
    Export(String, String),
    Echo(String),
    Unset(String),
    Set(String, String),
    ReadOnly(String, String),
    Exec(String, Vec<String>),
    Eval(String),
    Exit,
}

pub fn is_builtin(cmd: &str) -> bool {
    matches!(cmd, "cd" | "export" | "unset" | "set" | "readonly" | "exec" | "eval" | "exit" | "echo")
}

fn parse_builtin_cmd(cmd: &str) -> Option<BuiltIn> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    match parts[0] {
        "cd" => Some(BuiltIn::Cd(parts[1].to_string())),
        "export" => Some(BuiltIn::Export(parts[1].to_string(), parts[2].to_string())),
        "echo" => Some(BuiltIn::Echo(parts[1..].join(" "))),
        "unset" => Some(BuiltIn::Unset(parts[1].to_string())),
        "set" => Some(BuiltIn::Set(parts[1].to_string(), parts[2].to_string())),
        "readonly" => Some(BuiltIn::ReadOnly(parts[1].to_string(), parts[2].to_string())),
        "exec" => Some(BuiltIn::Exec(parts[1].to_string(), parts[2..].iter().map(|s| s.to_string()).collect())),
        "eval" => Some(BuiltIn::Eval(parts[1..].join(" "))),
        "exit" => Some(BuiltIn::Exit),
        _ => None,
    }
}

pub fn run_cmd(cmd: &str, state: &mut ShellState) -> Result<(), ShellErrors>   {
    if let Some(builtin) = parse_builtin_cmd(cmd) {
        return run_builtin_cmd(builtin, state);
    }

    Err(ShellErrors::NotSupportedCmd(cmd.to_string()))
}

pub fn run_builtin_cmd(cmd: BuiltIn, state: &mut ShellState) -> Result<(), ShellErrors> {
    match cmd {
        BuiltIn::Cd(path) => {
            state.change_dir("cd", &path)
        },
        BuiltIn::Export(key, value) => {
            state.set_env_var(key, value);
            Ok(())
        },
        BuiltIn::Echo(val) => {
            println!("{}", val);
            if val.starts_with("$") {
                let var_name = &val[1..];
                if let Some(var_value) = state.get_env_var(var_name) {
                    println!("{}", var_value);
                } else {
                    println!("{}: Undefined variable", var_name);
                }
            }
            Ok(())
        },
        BuiltIn::Unset(key) => {
            state.unset_env_var(&key);
            Ok(())
        },
        BuiltIn::Set(key, value) => {
            state.set_env_var(key, value);
            Ok(())
        },
        BuiltIn::ReadOnly(key, value) => {
            state.set_env_var(key, value);
            Ok(())
        }
        BuiltIn::Exec(cmd, args) => {
            println!("Executing command: {} with args: {:?}", cmd, args);
            Ok(())
        },
        BuiltIn::Eval(cmd) => {
            println!("Evaluating command: {}", cmd);
            Ok(())
        },
        BuiltIn::Exit => {
            println!("Exiting shell...");
            std::process::exit(0);
        }
    }
}