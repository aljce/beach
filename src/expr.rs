use std::fmt;
use std::fmt::{Display, Formatter};
use itertools::Itertools;

pub enum Arg<'a> {
    Short(&'a str),
    Long(&'a str),
    Name(&'a str),
}

impl<'a> Display for Arg<'a> {
    fn fmt(&self, format: &mut Formatter) -> fmt::Result {
        use self::Arg::*;
        match *self {
            Short(c) => write!(format, "-{}", c),
            Long(s)  => write!(format, "--{}", s),
            Name(s)  => write!(format, "{}", s)
        }
    }
}

pub enum Expr<'a> {
    Command {
        name: &'a str,
        args: Vec<Arg<'a>>
    },
}

impl<'a> Display for Expr<'a> {
    fn fmt(&self, format: &mut Formatter) -> fmt::Result {
        use self::Expr::*;
        match *self {
            Command { name, ref args } => {
                let rest = args.iter().join(" ");
                write!(format, "{} {}", name, rest)
            },
        }
    }
}

