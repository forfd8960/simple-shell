use rustyline::{DefaultEditor, error::ReadlineError};

// run shell loop, read input, parse, execute commands
pub fn run_shell() -> anyhow::Result<()>{
    println!("Running shell...");

    let mut rl = DefaultEditor::new()?;
    loop {
        let readline = rl.readline("osh>> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;
                println!("Line: {}", line);
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
                break
            }
        }
    }

    Ok(())
}