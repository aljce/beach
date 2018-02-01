use std::fmt;
use std::fmt::{Display, Formatter};
use itertools::Itertools;
use nom::{space, Err, ErrorKind};

#[derive(Clone, Debug, PartialEq)]
pub enum Arg<'a> {
    Short(&'a str),
    Long(&'a str),
    Name(&'a str)
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
    Command {
        name: &'a str,
        args: Vec<Arg<'a>>
    },
    Sequence {
        left:  Box<Expr<'a>>,
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
            Command { name, ref args } => {
                let rest = args.iter().join(" ");
                write!(format, "{} {}", name, rest)
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
    arg<&str, Arg>,
    return_error!(
        ErrorKind::Custom(0),
        alt_complete!(
            do_parse!(tag!("--") >> arg: string >> (Arg::Long(arg))) |
            do_parse!(char!('-') >> arg: string >> (Arg::Short(arg))) |
            map!(string, Arg::Name)
        )
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
    strict_args<&str, Vec<Arg>>,
    separated_list_complete!(space, arg)
);

named!(
    args<&str, Vec<Arg>>,
    map!(
        opt!(strict_args),
        |res| res.unwrap_or(vec![])
    )
);

named!(
    command<&str, Expr>,
    do_parse!(
        name: string >>
        opt!(space) >>
        args: args >>
        (Expr::Command { name, args })
    )
);

named!(
    sequence<&str, Expr>,
    do_parse!(
        left: command >>
        opt!(space) >>
        op: operator >>
        opt!(space) >>
        right: command >>
        (Expr::Sequence { left: Box::new(left), op, right: Box::new(right) })
    )
);

named!(
    expr<&str, Expr>,
    alt_complete!( sequence | command )
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

    #[test]
    fn arg() {
        parses_to(super::arg("--foo"), Arg::Long("foo"));
        parses_to(super::arg("-foo"), Arg::Short("foo"));
        parses_to(super::arg("foo"), Arg::Name("foo"));
    }

    fn total_to(s: &str, correct: Expr) {
        parses_to(super::total(s), correct)
    }

    #[test]
    fn command() {
        let ping = Expr::Command {
            name: "ping",
            args: vec![]
        };
        total_to("ping ", ping);
        let ping_args = Expr::Command {
            name: "ping",
            args: vec![Arg::Short("t"), Arg::Name("5")]
        };
        total_to("ping -t 5", ping_args);
    }

    #[test]
    fn total() {
        let find = Expr::Command {
            name: "find",
            args: vec![
                Arg::Short("t"),
                Arg::Name("f"),
                Arg::Long("name"),
                Arg::Name("result")
            ]
        };
        let cat = Expr::Command {
            name: "cat",
            args: vec![]
        };
        let comm = Expr::Sequence {
            left: Box::new(find),
            op: Operator::Pipe,
            right: Box::new(cat)
        };
        let res = Expr::Redirect {
            expr: Box::new(comm),
            file: "file.txt"
        };
        total_to("find -t f --name result | cat > file.txt", res);
    }
}
