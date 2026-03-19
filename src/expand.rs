use crate::{Command, ListSeparator, LogicalOp, SimpleCommand};

pub fn expand_commands(commands: Vec<Command>) -> Vec<Command> {
    let mut new_commands: Vec<Command> = Vec::new();
    for cmd in commands {
        match cmd {
            Command::AndOr {
                left,
                operator,
                right,
            } => {
                let new_l_r = expand_commands(vec![*left, *right]);

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
                    expand_commands(vec![*left, *right.unwrap()])
                } else {
                    expand_commands(vec![*left])
                };

                new_commands.push(expand_list(separator, new_l_r));
            }

            Command::Pipeline(cmds) => {
                new_commands.extend(expand_commands(cmds));
            }

            Command::Simple(cmd) => {
                new_commands.push(expand_simple(cmd));
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

fn expand_simple(simple: SimpleCommand) -> Command {
    let expand_cmds = simple
        .cmds
        .iter()
        .map(|cmd| shellexpand::full(cmd).unwrap().to_string())
        .collect::<Vec<String>>();

    Command::Simple(SimpleCommand {
        cmds: expand_cmds,
        io_rds: simple.io_rds,
    })
}

#[cfg(test)]
mod tests {
    use crate::{Command, SimpleCommand, expand::expand_simple};

    #[test]
    fn test_expand_simple() {
        let sim = expand_simple(SimpleCommand {
            cmds: vec!["echo".to_string(), "$HOME".to_string()],
            io_rds: vec![],
        });

        assert_eq!(
            sim,
            Command::Simple(SimpleCommand {
                cmds: vec!["echo".to_string(), "/Users/xxx".to_string()],
                io_rds: vec![]
            })
        );
    }
}
