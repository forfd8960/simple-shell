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
        let child_p = if let Some(simple) = left.as_simple() {
            Some(self.execute_simple(simple)?)
        } else {
            None
        };

        let success = if let Some(mut child) = child_p {
            println!("Started background process with PID: {}", child.id());

            child
                .wait()
                .expect("Failed to wait on child process")
                .success()
        } else {
            false
        };

        if op == LogicalOp::And {
            if success {
                println!("Command succeeded, executing next command...");
                self.execute_cmd(right)?;
            }
        } else {
            if !success {
                println!("Command failed, executing next command...");
                self.execute_cmd(right)?;
            }
        }

        Ok(())
    }

    fn execute_pipe(&mut self, cmds: Vec<Command>) -> Result<(), ShellErrors> {
        if cmds.len() == 0 {
            return Ok(());
        }

        let cmd_len = cmds.len();

        let mut cmd1_child = if let Some(simple) = cmds[0].as_simple() {
            let mut pipe_cmd1 = self.build_pipe_cmd(simple.clone());
            pipe_cmd1.stdout(Stdio::piped());

            let pipe_cmd1_child = pipe_cmd1
                .spawn()
                .expect("failed to execute first command in pipeline");

            pipe_cmd1_child
        } else {
            return Ok(());
        };

        for (idx, cmd) in cmds[1..].iter().enumerate() {
            if let Command::Simple(simple) = cmd {
                let mut pipe_cmd2 = self.build_pipe_cmd(simple.clone());
                pipe_cmd2.stdin(Stdio::piped());

                if idx == cmd_len - 2 {
                    let output2 = pipe_cmd2
                        .stdin(cmd1_child.stdout.expect("Failed to get stdout"))
                        .output()
                        .expect(&format!("failed to run command: {:?}", simple.cmds));

                    let result = str::from_utf8(&output2.stdout).expect("failed to decode output");
                    println!("{}", result);
                    break;
                } else {
                    pipe_cmd2.stdout(Stdio::piped());

                    let cmd2_child = pipe_cmd2
                        .stdin(cmd1_child.stdout.expect("Failed to get stdout"))
                        .spawn()
                        .expect(&format!(
                            "failed to execute command: {:?} in pipeline",
                            simple.cmds
                        ));

                    cmd1_child = cmd2_child;
                }
            }
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
