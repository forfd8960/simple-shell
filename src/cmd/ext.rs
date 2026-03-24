use std::cell::RefCell;
use std::fs::OpenOptions;
use std::process::Child;
use std::process::Command as OsCommand;
use std::process::Stdio;

use crate::RedirectOp;
use crate::{
    Command, ListSeparator, LogicalOp, SimpleCommand, errors::ShellErrors, state::ShellState,
};

pub struct CommandRunner<'a> {
    pub state: RefCell<&'a ShellState>,
}

impl<'a> CommandRunner<'a> {
    pub fn new(state: RefCell<&'a ShellState>) -> Self {
        Self { state }
    }

    pub fn run_ext_cmds(&mut self, cmds: Vec<Command>) -> Result<(), ShellErrors> {
        for cmd in cmds {
            self.execute_cmd(cmd)?;
        }
        Ok(())
    }

    fn execute_cmd(&mut self, cmd: Command) -> Result<(), ShellErrors> {
        match cmd {
            Command::List {
                left,
                separator,
                right,
            } => {
                if right.is_some() {
                    return self.execute_list(*left, separator, Some(*right.unwrap()));
                } else {
                    return self.execute_list(*left, separator, None);
                }
            }
            Command::AndOr {
                left,
                operator,
                right,
            } => {
                return self.execute_and_or(*left, operator, *right);
            }

            Command::Pipeline(cmds) => {
                return self.execute_pipe(cmds);
            }

            Command::Simple(simple) => {
                let mut child_process = self.execute_simple(&simple)?;
                child_process
                    .wait()
                    .expect("Failed to wait on child process");
                Ok(())
            }
        }
    }

    fn execute_list(
        &mut self,
        left: Command,
        op: ListSeparator,
        right: Option<Command>,
    ) -> Result<(), ShellErrors> {
        let child_p = if let Some(simple) = left.as_simple() {
            Some(self.execute_simple(simple)?)
        } else {
            None
        };

        if op == ListSeparator::Async {
            if let Some(child) = child_p {
                println!("Started background process with PID: {}", child.id());
            }
        } else {
            if let Some(mut child) = child_p {
                child.wait().expect("Failed to wait on child process");
            }
        }

        if let Some(right_cmd) = right {
            self.execute_cmd(right_cmd)?;
        }

        Ok(())
    }

    fn execute_and_or(
        &mut self,
        left: Command,
        op: LogicalOp,
        right: Command,
    ) -> Result<(), ShellErrors> {
        let success = match left.as_simple() {
            Some(simple) => {
                let mut child = self.execute_simple(simple)?;
                println!("Started background process with PID: {}", child.id());
                child
                    .wait()
                    .expect("Failed to wait on child process")
                    .success()
            }
            None => false,
        };

        let should_run_right = match op {
            LogicalOp::And => success,
            LogicalOp::Or => !success,
        };

        if should_run_right {
            self.execute_cmd(right)?;
        }

        Ok(())
    }

    fn execute_pipe(&mut self, cmds: Vec<Command>) -> Result<(), ShellErrors> {
        let (first, rest) = match cmds.split_first() {
            Some(parts) => parts,
            None => return Ok(()),
        };

        let first_simple = match first.as_simple() {
            Some(simple) => simple.clone(),
            None => return Ok(()),
        };

        let mut previous_child = {
            let mut first_process = self.build_pipe_cmd(first_simple);
            first_process.stdout(Stdio::piped());
            first_process
                .spawn()
                .expect("failed to execute first command in pipeline")
        };

        for (idx, cmd) in rest.iter().enumerate() {
            let simple = match cmd {
                Command::Simple(simple) => simple,
                _ => continue,
            };

            let mut next_process = self.build_pipe_cmd(simple.clone());
            let previous_stdout = previous_child.stdout.take().expect("Failed to get stdout");
            next_process.stdin(previous_stdout);

            let is_last_stage = idx == rest.len() - 1;

            if is_last_stage {
                let output = next_process
                    .output()
                    .expect(&format!("failed to run command: {:?}", simple.cmds));

                let result = str::from_utf8(&output.stdout).expect("failed to decode output");
                println!("{}", result);
                break;
            }

            next_process.stdout(Stdio::piped());
            previous_child = next_process
                .spawn()
                .expect(&format!("failed to execute command: {:?} in pipeline", simple.cmds));
        }

        Ok(())
    }

    fn execute_simple(&mut self, simple: &SimpleCommand) -> Result<Child, ShellErrors> {
        let program = &simple.cmds[0];
        let args = &simple.cmds[1..];

        let mut os_cmd = OsCommand::new(program);
        os_cmd.args(args);

        for r in &simple.io_rds {
            match r.operator {
                RedirectOp::Input => {
                    let f = OpenOptions::new().read(true).open(&r.target)?;
                    os_cmd.stdin(Stdio::from(f));
                }
                RedirectOp::Output => {
                    let f = OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(&r.target)?;
                    os_cmd.stdout(Stdio::from(f));
                }
                RedirectOp::Append => {
                    let f = OpenOptions::new()
                        .write(true)
                        .create(true)
                        .append(true)
                        .open(&r.target)?;
                    os_cmd.stdout(Stdio::from(f));
                }
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

    fn build_pipe_cmd(&mut self, cmd: SimpleCommand) -> OsCommand {
        let program = &cmd.cmds[0];
        let args = &cmd.cmds[1..];

        let mut os_cmd = OsCommand::new(program);
        os_cmd.args(args);
        os_cmd
    }
}
