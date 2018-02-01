extern crate itertools;
#[macro_use]
extern crate nom;

mod shell;
use shell::expr;

pub fn example() {
    let e = expr::parse("echo hello world ").unwrap();
    println!("> {}",e);
    shell::exec(e);
}
