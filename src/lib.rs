extern crate itertools;
#[macro_use]
extern crate nom;
extern crate rustyline;

use rustyline::Editor;
use rustyline::error::ReadlineError;
mod shell;
use shell::{Env, Result};

pub fn repl() {
    let prompt = "> "; // TODO: Config?
    let env = Env::new();
    loop {
        let mut rl = Editor::<()>::new();
        let readline = rl.readline(prompt);
        match readline {
            Ok(ref line) if line.is_empty() => {},
            Ok(ref line) => {
                let mut fixed = line.clone();
                fixed.push(' '); // TODO: This a huge hack to fix an egde case in the parser FIXME
                match shell::parse(&fixed) {
                    Err(err) => {
                        let no_newlines = line.chars().filter(|c| *c != '\n').collect::<String>();
                        println!("ERROR: Could not parse ({}) because {}", no_newlines, err)
                    },
                    Ok(e) => {
                        if let Result::Exit = shell::exec(&env, e) {
                            break
                        };
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
