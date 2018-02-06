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
use shell::{Env, ProcessErr};

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
    let history_file = home_dir().expect("no home directory").join(".beach_history");
    if rl.load_history(&history_file).is_err() {
        println!("no history file creating[{}]...", history_file.to_string_lossy());
        let mut file = File::create(&history_file).unwrap();
        file.write_all(b"").unwrap();
    }
    loop {
        let readline = rl.readline(prompt);
        match readline {
            Ok(ref line) if line.is_empty() => {}
            Ok(ref line) => {
                match shell::parse(&line) {
                    Err(err) => {
                        let no_newlines = line.chars().filter(|c| *c != '\n').collect::<String>();
                        eprintln!("ERROR: could not parse ({}) because {}", no_newlines, err)
                    },
                    Ok(e) => {
                        rl.add_history_entry(line.as_ref());
                        if let Err(process_err) = shell::exec(&env, e) {
                            match process_err {
                                ProcessErr::Continue => {},
                                ProcessErr::Exit => break,
                                ProcessErr::Error(io_err) => {
                                    eprintln!("ERROR: {}", io_err);
                                }
                                ProcessErr::Pipe => {
                                    eprintln!("ERROR: could not aquire a stdin or stdout");
                                }
                            }
                        };
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                eprintln!("CTRL-C");
                break
            }
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                eprintln!("ERROR: {}", err);
                break
            }
        }
    }
    rl.save_history(&history_file).unwrap();
}
