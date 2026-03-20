use std::fs::OpenOptions;
use std::process::Child;
use std::process::Command as OsCommand;
use std::process::Stdio;

use crate::RedirectOp;
use crate::{Command, ListSeparator, LogicalOp, SimpleCommand, errors::ShellErrors, state::ShellState};

pub struct CommandRunner<'a> {
    pub state: &'a ShellState,
}

impl<'a> CommandRunner<'a> {
    pub fn new(state: &'a ShellState) -> Self {
        Self { state }
    }

    pub fn run_ext_cmds(&mut self, cmds: Vec<Command>) -> Result<(), ShellErrors> {
        Ok(())
    }

    fn execute_cmd(&mut self, cmd: Command) -> Result<(), ShellErrors> {
        match cmd {
            Command::List { left, separator, right } => {
                if right.is_some() {
                    return self.execute_list(*left, separator, Some(*right.unwrap()));
                } else {
                    return self.execute_list(*left, separator, None);
                }
            }
            Command::AndOr { left, operator, right } => {
                return self.execute_and_or(*left, operator, *right);
            }
            _ => Ok(())
        }
    }

    fn execute_list(&mut self, left: Command, op: ListSeparator, right: Option<Command>) -> Result<(), ShellErrors> {
        Ok(())
    }

    fn execute_and_or(&mut self, left: Command, op: LogicalOp, right: Command) -> Result<(), ShellErrors> {
        Ok(())
    }

    fn execute_pipe(&mut self, cmds: Vec<Command>) -> Result<(), ShellErrors> {
        Ok(())
    }

    fn execute_simple(&mut self, simple: SimpleCommand) -> Result<Child, ShellErrors> {
        let program = &simple.cmds[0];
        let args = &simple.cmds[1..];

        let mut os_cmd = OsCommand::new(program);
        os_cmd.args(args);

        for r in simple.io_rds {
            match r.operator {
                RedirectOp::Input => {
                    let f = OpenOptions::new().read(true).open(&r.target)?;
                    os_cmd.stdin(Stdio::from(f));
                },
                RedirectOp::Output => {
                    let f = OpenOptions::new().write(true).create(true).truncate(true).open(&r.target)?;
                    os_cmd.stdout(Stdio::from(f));
                },
                RedirectOp::Append => {
                    let f = OpenOptions::new().write(true).create(true).append(true).open(&r.target)?;
                    os_cmd.stdout(Stdio::from(f));
                },
            }
        }

        match os_cmd.spawn() {
            Ok(child) => Ok(child),
            Err(e) => {
                eprintln!("execute cuild failed");
                Err(ShellErrors::IoError(e))
            }
        }
    }
}
