use glob::glob;
use regex::Regex;

use crate::{Command, ListSeparator, SimpleCommand, state::ShellState};

pub fn expand_commands(commands: Vec<Command>, state: &ShellState) -> Vec<Command> {
    let mut new_commands: Vec<Command> = Vec::new();
    for cmd in commands {
        match cmd {
            Command::AndOr {
                left,
                operator,
                right,
            } => {
                let new_l_r = expand_commands(vec![*left, *right], state);

                new_commands.push(Command::AndOr {
                    left: Box::new(new_l_r[0].clone()),
                    operator,
                    right: Box::new(new_l_r[1].clone()),
                });
            }
            Command::List {
                left,
                separator,
                right,
            } => {
                let new_l_r = if right.is_some() {
                    expand_commands(vec![*left, *right.unwrap()], state)
                } else {
                    expand_commands(vec![*left], state)
                };

                new_commands.push(expand_list(separator, new_l_r));
            }

            Command::Pipeline(cmds) => {
                new_commands.push(Command::Pipeline(expand_commands(cmds, state)));
            }

            Command::Simple(cmd) => {
                new_commands.push(expand_simple(cmd, state));
            }
        }
    }

    new_commands
}

fn expand_list(op: ListSeparator, new_l_r: Vec<Command>) -> Command {
    let right = if new_l_r.len() >= 2 {
        Some(Box::new(new_l_r[1].clone()))
    } else {
        None
    };

    Command::List {
        left: Box::new(new_l_r[0].clone()),
        separator: op,
        right,
    }
}

fn expand_simple(simple: SimpleCommand, state: &ShellState) -> Command {
    if simple.cmds[0] == "grep" {
        return Command::Simple(simple);
    }

    let expand_cmds = simple
        .cmds
        .iter()
        .map(|cmd| shellexpand::full_with_context(cmd, home_dir, |name| get_env(name, state)).unwrap().to_string())
        .collect::<Vec<String>>();

    Command::Simple(SimpleCommand {
        cmds: expand_path(expand_cmds),
        io_rds: simple.io_rds,
    })
}

fn home_dir() -> Option<String> { Some(std::env::var("HOME").unwrap_or_default()) }

fn get_env(name: &str, state: &ShellState) -> Result<Option<&'static str>, &'static str> {
    let res = state.get_env_var(name);
    match res {
        Some(val) => Ok(Some(Box::leak(val.clone().into_boxed_str()))),
        None => Ok(None),
    }
}

fn expand_path(expand_cmds: Vec<String>) -> Vec<String> {
    let mut new_cmds = Vec::new();

    let is_find_cmd = expand_cmds.first().is_some_and(|cmd| cmd == "find");
    let find_pattern_opts = [
        "-name",
        "-iname",
        "-path",
        "-wholename",
        "-regex",
        "-iregex",
    ];
    let mut previous_arg: Option<String> = None;

    for cmd in expand_cmds.into_iter() {
        let current_arg = cmd.clone();
        let cmd_str = cmd.as_str();
        println!("Expanding cmd: {}", cmd_str);

        let is_find_pattern_arg = is_find_cmd
            && previous_arg
                .as_deref()
                .is_some_and(|arg| find_pattern_opts.contains(&arg));

        if is_find_pattern_arg {
            previous_arg = Some(current_arg);
            new_cmds.push(cmd);
            continue;
        }

        if is_glob_pattern(&cmd_str) {
            println!("Found glob pattern: {}", cmd_str);

            for entry in glob(cmd_str).expect("Failed to read glob pattern") {
                match entry {
                    Ok(path) => new_cmds.push(path.to_string_lossy().to_string()),
                    Err(e) => println!("Glob error: {:?}", e),
                }
            }
        } else {
            new_cmds.push(cmd);
        }

        previous_arg = Some(current_arg);
    }

    new_cmds
}

fn is_glob_pattern(word: &str) -> bool {
    // Matches *, ?, or [ that is either at the beginning of the string
    // or preceded by a character that is NOT a backslash
    let re = Regex::new(r"(?:^|[^\\])[*?\[]").unwrap();
    re.is_match(word)
}

#[cfg(test)]
mod tests {
    use crate::{
        Command, ListSeparator, SimpleCommand,
        expand::{expand_commands, expand_simple}, state::ShellState,
    };

    #[test]
    fn test_expand_simple() {
        let state = ShellState::new();
        let sim = expand_simple(SimpleCommand {
            cmds: vec!["echo".to_string(), "$HOME".to_string()],
            io_rds: vec![],
        }, &state);

        assert_eq!(
            sim,
            Command::Simple(SimpleCommand {
                cmds: vec!["echo".to_string(), "/Users/alexz".to_string()],
                io_rds: vec![]
            })
        );
    }

    #[test]
    fn test_expand_list() {
        let state = ShellState::new();
        let sim = expand_commands(vec![Command::List {
            left: Box::new(Command::Simple(SimpleCommand {
                cmds: vec!["echo".to_string(), "$PWD".to_string()],
                io_rds: vec![],
            })),
            separator: ListSeparator::Async,
            right: None,
        }], &state);

        assert_eq!(
            sim,
            vec![Command::List {
                left: Box::new(Command::Simple(SimpleCommand {
                    cmds: vec![
                        "echo".to_string(),
                        "/Users/alexz/Documents/Code/Github/simple-shell".to_string()
                    ],
                    io_rds: vec![],
                })),
                separator: ListSeparator::Async,
                right: None,
            }]
        );
    }

    #[test]
    // grep -rl "tests" src/*.rs
    fn test_expand_glob() {
        let state = ShellState::new();
        let sim = expand_simple(SimpleCommand {
            cmds: "find . -name \"*.rs\""
                .split(" ")
                .map(|w| w.to_string())
                .collect(),
            io_rds: vec![],
        }, &state);

        assert_eq!(
            sim,
            Command::Simple(SimpleCommand {
                cmds: vec![
                    "find".to_string(),
                    ".".to_string(),
                    "-name".to_string(),
                    "\"*.rs\"".to_string(),
                ],
                io_rds: vec![],
            })
        );
    }

    #[test]
    // grep -rl "tests" src/*.rs
    fn test_expand_glob1() {
        let state = ShellState::new();
        let sim = expand_simple(SimpleCommand {
            cmds: r#"grep -rl "tests" src/*.rs"#.split(" ").map(|w| w.to_string()).collect(),
            io_rds: vec![],
        }, &state);

        assert_eq!(
            sim,
            Command::Simple(SimpleCommand {
                cmds: vec![
                    "grep".to_string(),
                    "-rl".to_string(),
                    "\"tests\"".to_string(),
                    "src/*.rs".to_string(),
                ],
                io_rds: vec![],
            })
        );
    }
}
