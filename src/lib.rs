extern crate itertools;
#[macro_use]
extern crate nom;
extern crate rustyline;

use rustyline::Editor;
use rustyline::error::ReadlineError;
mod shell;

pub fn repl() {
    let prompt = "> "; // TODO: Config?
    loop {
        let mut rl = Editor::<()>::new();
        let readline = rl.readline(prompt);
        match readline {
            Ok(ref line) if line.is_empty() => {},
            Ok(ref line) => {
                match shell::parse(line) {
                    Err(err) => {
                        let no_newlines = line.chars().filter(|c| *c != '\n').collect::<String>();
                        println!("ERROR: Could not parse ({}) because {}", no_newlines, err)
                    },
                    Ok(e) => {
                        shell::exec(e);
                    }
                }
            }
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
}
