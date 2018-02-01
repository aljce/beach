use std::fmt;
use std::fmt::{Display, Formatter};
use std::ffi::OsStr;
use itertools::Itertools;
use nom::{space, Err, ErrorKind};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Program<'a> {
    Cd,
    Exit,
    Other(&'a str)
}

impl<'a> AsRef<OsStr> for Program<'a> {
    fn as_ref(&self) -> &OsStr {
        use self::Program::*;
        let s = match *self {
            Cd => "cd",
            Exit => "exit",
            Other(name) => name
        };
        s.as_ref()
    }
}

impl<'a> Display for Program<'a> {
    fn fmt(&self, format: &mut Formatter) -> fmt::Result {
        use self::Program::*;
        match *self {
            Cd => write!(format, "cd"),
            Exit => write!(format, "exit"),
            Other(name) => write!(format, "{}", name)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Process<'a> {
    pub name: Program<'a>,
    pub args: Vec<&'a str>
}

impl<'a> Display for Process<'a> {
    fn fmt(&self, format: &mut Formatter) -> fmt::Result {
        let rest = self.args.iter().join(" ");
        write!(format, "{} {}", self.name, rest)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Operator {
    Pipe,
    Or,
    And
}

impl Display for Operator {
    fn fmt(&self, format: &mut Formatter) -> fmt::Result {
        use self::Operator::*;
        match *self {
            Pipe => write!(format, "|"),
            Or   => write!(format, "||"),
            And  => write!(format, "&&")
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr<'a> {
    Base(Process<'a>),
    Sequence {
        left:  Process<'a>,
        op:    Operator,
        right: Box<Expr<'a>>
    },
    Redirect {
        expr: Box<Expr<'a>>,
        file: &'a str
    },
}

impl<'a> Display for Expr<'a> {
    fn fmt(&self, format: &mut Formatter) -> fmt::Result {
        use self::Expr::*;
        match *self {
            Base(ref process) => {
                write!(format, "{}", process)
            },
            Sequence { ref left, op, ref right } => {
                write!(format, "{} {} {}", left, op, right)
            },
            Redirect { ref expr, file } => {
                write!(format, "{} > {}", expr, file)
            },
        }
    }
}

named!(
    string<&str, &str>,
    take_till_s!(|c| c == ' ' || c == '|' || c == '&' || c == '>')
);

named!(
    program<&str, Program>,
    alt_complete!(
        value!(Program::Cd,   tag_s!("cd")) |
        value!(Program::Exit, tag_s!("exit")) |
        map!(string, Program::Other)
    )
);

named!(
    strict_args<&str, Vec<&str>>,
    separated_list_complete!(space, string)
);

named!(
    args<&str, Vec<&str>>,
    map!(
        opt!(strict_args),
        |res| res.unwrap_or(vec![])
    )
);

named!(
    process<&str, Process>,
    do_parse!(
        name: program >>
        opt!(space) >>
        args: args >>
        (Process { name, args })
    )
);

named!(
    operator<&str, Operator>,
    return_error!(
        ErrorKind::Custom(1),
        alt_complete!(
            value!(Operator::Pipe, char!('|')) |
            value!(Operator::Or,   tag_s!("||")) |
            value!(Operator::And,  tag_s!("&&"))
        )
    )
);

named!(
    sequence<&str, Expr>,
    do_parse!(
        left: process >>
        opt!(space) >>
        op: operator >>
        opt!(space) >>
        right: expr >>
        (Expr::Sequence { left, op, right: Box::new(right) })
    )
);

named!(
    expr<&str, Expr>,
    alt_complete!( sequence | map!(process, Expr::Base) )
);

named!(
    redirect<&str, Expr>,
    do_parse!(
        expr: expr >>
        opt!(space) >>
        char!('>') >>
        opt!(space) >>
        file: string >>
        (Expr::Redirect { expr: Box::new(expr), file })
    )
);

named!(
    total<&str, Expr>,
    alt_complete!( redirect | expr )
);

pub fn parse<'a>(s: &'a str) -> Result<Expr<'a>, Err> {
    total(s).to_result()
}

#[cfg(test)]
mod tests {
    use std::fmt::{Debug};
    use nom::{IResult};

    fn parses_to<A>(res: IResult<&str, A>, correct: A)
        where A: PartialEq<A> + Debug
    {
        match res {
            IResult::Done(i,o) => {
                if o != correct {
                    panic!("{:?} does not equal {:?} and the parse had these leftovers: {}", o, correct, i)
                }
            },
            IResult::Error(err) => panic!("error: {:?}", err),
            IResult::Incomplete(needed) => panic!("needed: {:?}", needed)
        }
    }

    use super::*;

    fn total_to(s: &str, correct: Expr) {
        parses_to(super::total(s), correct)
    }

    #[test]
    fn command() {
        let cd = Expr::Base (
            Process {
                name: Program::Cd,
                args: vec![]
            }
        );
        total_to("cd ", cd);
        let ping = Expr::Base(
            Process {
                name: Program::Other("ping"),
                args: vec![]
            }
        );
        total_to("ping ", ping);
        let ping_args = Expr::Base(
            Process {
                name: Program::Other("ping"),
                args: vec!["-t", "5"]
            }
        );
        total_to("ping -t 5", ping_args);
    }

    #[test]
    fn total() {
        let find = Process {
            name: Program::Other("find"),
            args: vec!["-t", "f", "--name", "result"]
        };
        let cat = Process {
            name: Program::Other("cat"),
            args: vec![]
        };
        let comm = Expr::Sequence {
            left: find,
            op: Operator::Pipe,
            right: Box::new(Expr::Base(cat))
        };
        let res = Expr::Redirect {
            expr: Box::new(comm),
            file: "file.txt"
        };
        total_to("find -t f --name result | cat > file.txt", res);
    }
}
