use std::fmt;
use std::fmt::{Display, Formatter};

enum Arg<'a> {
    Short(&'a str),
    Long(&'a str),
    Name(&'a str),
}

enum Expr<'a> {
    Command {
        name: &'a str,
        args: Vec<Arg<'a>>
    },
}

impl<'a> Display for Expr<'a> {
    fn fmt(&self, format: &mut Formatter) -> fmt::Result {
        use self::Expr::*;
        match *self {
            Command { name, args } => write!(format, "{}", name),
        }
    }
}

fn main() {
    let x = Expr::Command {
        name: "ping",
        args: vec![],
    };
    println!("{}", x);
}
