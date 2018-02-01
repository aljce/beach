extern crate itertools;
#[macro_use]
extern crate nom;

use std::io;

mod shell;

pub fn repl() -> ! {
    loop {
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).expect("ERROR: Could not aquire stdin lock");
        let parsed = shell::parse(&buffer);
        match parsed {
            Err(err) => {
                let no_newlines = buffer.chars().filter(|c| *c != '\n').collect::<String>();
                println!("ERROR: Could not parse ({}) because {}", no_newlines, err)
            },
            Ok(e) => shell::exec(e)
        }
    }
}
