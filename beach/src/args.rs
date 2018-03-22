use std::str::FromStr;
use std::num::ParseIntError;
use std::string::ParseError;
use std::path::PathBuf;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use itertools::Itertools;
use frunk::hlist::*;
use frunk::coproduct::*;
use void::Void;
use umbrella::BlockNumber;
use umbrella::fs::INodeFlags;

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Err<E> {
    MissingArgument,
    Other(E)
}

impl<E> Err<E> {
    pub fn map<F, O>(self, f: F) -> Err<O>
    where F : FnOnce(E) -> O {
        match self {
            Err::MissingArgument => Err::MissingArgument,
            Err::Other(err) => Err::Other(f(err))
        }
    }
}

impl<E: Display> Display for Err<E> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match *self {
            Err::MissingArgument => writeln!(f, "Error: missing argument"),
            Err::Other(ref err)  => writeln!(f, "Error: {}", err)
        }
    }
}

impl<E: Error> Error for Err<E> {
    fn description(&self) -> &str {
        match *self {
            Err::MissingArgument => "missing argument",
            Err::Other(ref err) => err.description()
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            Err::MissingArgument => None,
            Err::Other(ref err) => err.cause()
        }
    }
}

impl<E> From<E> for Err<E> {
    fn from(err: E) -> Err<E> {
        Err::Other(err)
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Args {
    ptr: usize,
    pub vec: Vec<String>
}

impl Args {
    pub fn new(args: Vec<String>) -> Args {
        Args { ptr: 0, vec: args }
    }

    #[cfg(test)]
    pub fn empty() -> Args {
        Args::new(vec![])
    }

    pub fn pop<E>(&mut self) -> Result<String, Err<E>> {
        if self.ptr < self.vec.len() {
            let res = Ok(self.vec[self.ptr].clone());
            self.ptr += 1;
            res
        } else {
            Err(Err::MissingArgument)
        }
    }
}

impl Display for Args {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.vec.iter().join(" "))
    }
}

pub trait ParseArg: Sized {
    type Err;
    fn parse_arg(args: &mut Args) -> Result<Self, Err<Self::Err>>;
}

macro_rules! parse_arg {
    ($t:ty) => {
        impl ParseArg for $t {
            type Err = <$t as FromStr>::Err;
            fn parse_arg(args: &mut Args) -> Result<Self, Err<Self::Err>> {
                let arg = args.pop()?;
                Ok(<$t>::from_str(&arg)?)
            }
        }
    };
}

parse_arg!(u8);
parse_arg!(u16);
parse_arg!(u32);
parse_arg!(u64);
parse_arg!(String);

impl ParseArg for BlockNumber {
    type Err = <u64 as ParseArg>::Err;
    fn parse_arg(args: &mut Args) -> Result<Self, Err<Self::Err>> {
        u64::parse_arg(args).map(BlockNumber::new)
    }
}

pub struct ParseINodeFlagsError;

impl ParseArg for INodeFlags {
    type Err = ParseINodeFlagsError;
    fn parse_arg(args: &mut Args) -> Result<Self, Err<Self::Err>> {
        let arg = args.pop()?;
        INodeFlags::parse(&arg).map_err(|_| Err::Other(ParseINodeFlagsError))
            // Err(_)    => {
            //     eprintln!(
            //         "ERROR: Could not parse inode type [{}] please choose from [0fdsbD]",
            //         inode_type
            //     )
            // }
    }
}

impl ParseArg for PathBuf {
    type Err = Void;
    fn parse_arg(args: &mut Args) -> Result<Self, Err<Self::Err>> {
        args.pop().map(PathBuf::from)
    }
}

impl<A: ParseArg> ParseArg for Option<A> {
    type Err = A::Err;
    fn parse_arg(args: &mut Args) -> Result<Self, Err<Self::Err>> {
        let old_ptr = args.ptr;
        let res = match A::parse_arg(args) {
            Err(_) => {
                args.ptr = old_ptr;
                None
            }
            Ok(arg) => Some(arg)
        };
        Ok(res)
    }
}

pub trait Explain {
    fn explain(&self, prog: &str) -> String;
}

impl Explain for Void {
    fn explain(&self, _: &str) -> String {
        match *self { }
    }
}

impl Explain for ParseError {
    fn explain(&self, _: &str) -> String {
        match *self { }
    }
}

impl Explain for ParseIntError {
    fn explain(&self, prog: &str) -> String {
        format!("{} requires a non-negative integer which failed to parse because: {}", prog, self)
    }
}

impl Explain for ParseINodeFlagsError {
    fn explain(&self, prog: &str) -> String {
        format!(
            "{} requires a valid inode type please choose from [0fdsbD]",
            prog
        )
    }
}

impl Explain for CNil {
    fn explain(&self, _: &str) -> String {
        match *self { }
    }
}

impl<H: Explain, T: Explain> Explain for Coproduct<H, T> {
    fn explain(&self, prog: &str) -> String {
        match *self {
            Coproduct::Inl(ref err)  => err.explain(prog),
            Coproduct::Inr(ref rest) => rest.explain(prog)
        }
    }
}

pub trait Parse: HList {
    type Err;
    fn parse(args: Args) -> Result<Self, Err<Self::Err>>;
    fn parse_explain<F>(prog: &str, args: Args, f: F)
    where F: FnOnce(Self) -> (),
          Self::Err: Explain
    {
        match Self::parse(args) {
            Ok(parsed) => f(parsed),
            Err(err) => eprintln!("{}", err.map(|c| c.explain(prog)))
        }
    }
}

impl Parse for HNil {
    type Err = CNil;
    fn parse(_: Args) -> Result<Self, Err<Self::Err>> {
        Ok(HNil)
    }
}

impl<H: ParseArg, T: Parse> Parse for HCons<H, T> {
    type Err  = Coproduct<H::Err, T::Err>;
    fn parse(mut args: Args) -> Result<Self, Err<Self::Err>> {
        let head = H::parse_arg(&mut args).map_err(|e| e.map(Coproduct::Inl))?;
        let tail = T::parse(args).map_err(|e| e.map(Coproduct::Inr))?;
        Ok(HCons { head, tail })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_none() {
        assert_eq!(<Hlist![]>::parse(Args::empty()), Ok(hlist![]));
    }

    #[test]
    fn parse_many() {
        let vec = vec!["foobar".to_string(), "2".to_string(), "not-a-number".to_string()];
        let args = Args::new(vec);
        assert_eq!(
            <Hlist![String, u8, Option<u8>, String]>::parse(args),
            Ok(hlist!["foobar".to_string(), 2, None, "not-a-number".to_string()])
        );
    }
}
