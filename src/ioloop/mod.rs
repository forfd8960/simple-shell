use rustyline::{DefaultEditor, error::ReadlineError};

use crate::{cmd::run_cmd, state::ShellState};

// run shell loop, read input, parse, execute commands
pub fn run_shell() -> anyhow::Result<()>{
    println!("Running shell...");

    let mut rl = DefaultEditor::new()?;
    let mut state = ShellState::new();
    loop {
        let readline = rl.readline("osh>> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;
                state.append_history(line.clone());
                println!("Line: {}", line);

                if let Err(err) = run_cmd(&line, &mut state) {
                    println!("Error: {:?}", err);
                }
            },
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break
            },
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break
            },
            Err(err) => {
                println!("Error: {:?}", err);
                anyhow::bail!(err);
            }
        }
    }

    Ok(())
}