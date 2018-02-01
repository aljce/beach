extern crate itertools;
#[macro_use]
extern crate nom;
extern crate rustyline;

use std::env::home_dir;
use std::fs::File;
use std::io::Write;

use rustyline::{Cmd, Config, CompletionType, Editor, EditMode, KeyPress};
use rustyline::completion::FilenameCompleter;
use rustyline::error::ReadlineError;

mod shell;
use shell::{Env, Result};

pub fn repl() {
    let prompt = "> "; // TODO: Config?
    let env = Env::new();
    let config = Config::builder()
        .edit_mode(EditMode::Emacs)
        .completion_type(CompletionType::List)
        .max_history_size(1000)
        .history_ignore_space(true)
        .build();
    let c = FilenameCompleter::new();
    let mut rl = Editor::with_config(config);
    rl.set_completer(Some(c));
    rl.bind_sequence(KeyPress::Down, Cmd::HistorySearchForward);
    rl.bind_sequence(KeyPress::Up,   Cmd::HistorySearchBackward);
    let history_file = home_dir().expect("No home directory").join(".beach_history");
    if rl.load_history(&history_file).is_err() {
        println!("No history file creating...");
        let mut file = File::create(&history_file).unwrap();
        file.write_all(b"").unwrap();
    }
    loop {
        let readline = rl.readline(prompt);
        match readline {
            Ok(ref line) if line.is_empty() => {}
            Ok(ref line) => {
                let mut fixed = line.clone();
                fixed.push(' '); // TODO: This a huge hack to fix an egde case in the parser FIXME
                match shell::parse(&fixed) {
                    Err(err) => {
                        let no_newlines = line.chars().filter(|c| *c != '\n').collect::<String>();
                        println!("ERROR: Could not parse ({}) because {}", no_newlines, err)
                    },
                    Ok(e) => {
                        rl.add_history_entry(line.as_ref());
                        if let Result::Exit = shell::exec(&env, e) {
                            break
                        };
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break
            }
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                println!("ERROR: {}", err);
                break
            }
        }
    }
    rl.save_history(&history_file).unwrap();
}
