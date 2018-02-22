use std::str;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::ffi::OsStr;
use itertools::Itertools;
use nom::{space, multispace, Err, ErrorKind};

pub mod args;
pub use self::args::*;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Program<'a> {
    Cd,
    NewFS,
    Mount,
    BlockMap,
    AllocBlock,
    FreeBlock,
    INodeMap,
    AllocINode,
    FreeINode,
    Unmount,
    Exit,
    Other(&'a str)
}

impl<'a> AsRef<str> for Program<'a> {
    fn as_ref(&self) -> &str {
        use self::Program::*;
        match *self {
            Cd => "cd",
            NewFS => "newfs",
            Mount => "mount",
            BlockMap => "blockmap",
            AllocBlock => "alloc_block",
            FreeBlock => "free_block",
            INodeMap => "inode_map",
            AllocINode => "alloc_inode",
            FreeINode => "free_inode",
            Unmount => "unmount",
            Exit => "exit",
            Other(name) => name
        }
    }
}

impl<'a> AsRef<OsStr> for Program<'a> {
    fn as_ref(&self) -> &OsStr {
        let s : &str = self.as_ref();
        s.as_ref()
    }
}

impl<'a> Display for Program<'a> {
    fn fmt(&self, format: &mut Formatter) -> fmt::Result {
        let s : &str = self.as_ref();
        write!(format, "{}", s)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Process<'a> {
    pub name: Program<'a>,
    pub args: Args
}

impl<'a> Display for Process<'a> {
    fn fmt(&self, format: &mut Formatter) -> fmt::Result {
        let rest = self.args.vec.iter().join(" ");
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
        file: String
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
            Redirect { ref expr, ref file } => {
                write!(format, "{} > {}", expr, file)
            },
        }
    }
}

const DISALLOWED_CHARS : &'static str = " |&>\\\t\n\r\"";

named!(
    byte_string<&[u8]>,
    take_till!(|c : u8| DISALLOWED_CHARS.as_bytes().iter().any(|d| *d == c))
);

named!(
    string<&str>,
    map_res!(byte_string, str::from_utf8)
);

fn to_string(i: Vec<u8>) -> String {
  String::from_utf8_lossy(&i).into_owned()
}

named!(
    esc<String>,
    map!(
        escaped_transform!(
            is_not!(DISALLOWED_CHARS),
            '\\',
            alt!(
                  tag!(" ")  => { |_| " ".as_bytes() }
                | tag!("|")  => { |_| "|".as_bytes() }
                | tag!(">")  => { |_| ">".as_bytes() }
                | tag!("\\") => { |_| "\\".as_bytes() }
                | tag!("\t") => { |_| "\t".as_bytes() }
                | tag!("\n") => { |_| "\n".as_bytes() }
                | tag!("\r") => { |_| "\r".as_bytes() }
                | tag!("\"") => { |_| "\"".as_bytes() }
            )
        ),
        to_string
    )
);


named!(
    opt_space<()>,
    value!((), opt!(complete!(space)))
);

named!(
    program<Program>,
    alt_complete!(
        value!(Program::Cd,         tag_s!("cd")) |
        value!(Program::NewFS,      tag_s!("newfs")) |
        value!(Program::Mount,      tag_s!("mount")) |
        value!(Program::BlockMap,   tag_s!("blockmap")) |
        value!(Program::AllocBlock, tag_s!("alloc_block")) |
        value!(Program::FreeBlock,  tag_s!("free_block")) |
        value!(Program::INodeMap,   tag_s!("inode_map")) |
        value!(Program::AllocINode, tag_s!("alloc_inode")) |
        value!(Program::FreeINode,  tag_s!("free_inode")) |
        value!(Program::Unmount,    tag_s!("unmount")) |
        value!(Program::Exit,       tag_s!("exit")) |
        map!(string, Program::Other)
    )
);

named!(
    strict_args<Vec<String>>,
    separated_list_complete!(space, esc)
);

named!(
    args<Args>,
    map!(
        opt!(strict_args),
        |res| Args {
            vec: res.unwrap_or(vec![])
        }
    )
);

named!(
    process<Process>,
    do_parse!(
        name: program >>
        opt_space >>
        args: args >>
        (Process { name, args })
    )
);

named!(
    operator<Operator>,
    return_error!(
        ErrorKind::Custom(1),
        alt_complete!(
            value!(Operator::Or,   tag_s!("||")) |
            value!(Operator::And,  tag_s!("&&")) |
            value!(Operator::Pipe, char!('|'))
        )
    )
);

named!(
    sequence<Expr>,
    do_parse!(
        left: process >>
        opt_space >>
        op: operator >>
        opt_space >>
        right: expr >>
        (Expr::Sequence { left, op, right: Box::new(right) })
    )
);

named!(
    expr<Expr>,
    alt_complete!( sequence | map!(process, Expr::Base) )
);

named!(
    redirect<Expr>,
    do_parse!(
        expr: expr >>
        opt_space >>
        char!('>') >>
        opt_space >>
        file: esc >>
        (Expr::Redirect { expr: Box::new(expr), file })
    )
);

named!(
    total<Expr>,
    do_parse!(
        expr: alt_complete!( redirect | expr ) >>
        opt!(complete!(multispace)) >>
        (expr)
    )
);

pub fn parse<'a>(s: &'a str) -> Result<Expr<'a>, Err> {
    total(s.as_bytes()).to_result()
}

#[cfg(test)]
mod tests {
    use std::fmt::{Debug};
    use nom::{IResult};

    fn parses_to<A>(res: IResult<&[u8], A>, correct: A)
        where A: PartialEq<A> + Debug
    {
        match res {
            IResult::Done(i,o) => {
                if o != correct {
                    let i_str = str::from_utf8(i).unwrap();
                    panic!("{:?} does not equal {:?} and the parse had these leftovers: {}", o, correct, i_str)
                }
            },
            IResult::Error(err) => panic!("error: {:?}", err),
            IResult::Incomplete(needed) => panic!("needed: {:?}", needed)
        }
    }

    use super::*;

    fn total_to(s: &str, correct: Expr) {
        parses_to(super::total(s.as_bytes()), correct)
    }


    #[test]
    fn esc() {
        parses_to(super::esc("foobar".as_bytes()), "foobar".to_string());
        parses_to(super::esc("foobar ".as_bytes()), "foobar".to_string());
        parses_to(super::esc("foo\\ bar ".as_bytes()), "foo bar".to_string());
    }

    fn empty_args() -> Args {
        Args { vec: vec![] }
    }

    #[test]
    fn command() {
        let cd = Expr::Base (
            Process {
                name: Program::Cd,
                args: empty_args()
            }
        );
        total_to("cd", cd);
        let ping = Expr::Base(
            Process {
                name: Program::Other("ping"),
                args: empty_args()
            }
        );
        total_to("ping", ping);
        let args = Args {
            vec: vec!["-t".to_string(), "5".to_string()]
        };
        let ping_args = Expr::Base(
            Process { name: Program::Other("ping"), args }
        );
        total_to("ping -t 5", ping_args);
    }

    #[test]
    fn total() {
        let args = Args {
            vec: vec![ "-t".to_string()
                     , "f".to_string()
                     , "--name".to_string()
                     , "result".to_string() ]
        };
        let find = Process { name: Program::Other("find"), args };
        let cat = Process {
            name: Program::Other("cat"),
            args: empty_args()
        };
        let comm = Expr::Sequence {
            left: find,
            op: Operator::Pipe,
            right: Box::new(Expr::Base(cat))
        };
        let res = Expr::Redirect {
            expr: Box::new(comm),
            file: "file.txt".to_string()
        };
        total_to("find -t f --name result | cat > file.txt", res);
    }
}
