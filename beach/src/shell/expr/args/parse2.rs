use std::str::FromStr;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::borrow::Borrow;

use frunk::hlist::*;
use frunk::coproduct::*;
use void::Void;

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
        panic!()
    }
}

impl<E: Error> Error for Err<E> {
    fn description(&self) -> &str {
        panic!()
    }

    fn cause(&self) -> Option<&Error> {
        panic!()
    }
}

impl<E> From<E> for Err<E> {
    fn from(err: E) -> Err<E> {
        Err::Other(err)
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Args<'a> {
    ptr: usize,
    vec: Vec<&'a str>
}

impl<'a> Args<'a> {
    pub fn new<S>(args: &'a [S]) -> Args<'a>
    where S: AsRef<str>
    {
        Args {
            ptr: 0,
            vec: args.iter().map(|s| s.as_ref()).collect()
        }
    }

    pub fn pop<E>(&mut self) -> Result<&'a str, Err<E>> {
        if self.ptr < self.vec.len() {
            let res = Ok(self.vec[self.ptr]);
            self.ptr += 1;
            res
        } else {
            Err(Err::MissingArgument)
        }
    }
}

pub trait ParseArg: Sized {
    type Err;
    fn parse_arg(args: &mut Args) -> Result<Self, Err<Self::Err>>;
}

impl ParseArg for u8 {
    type Err = <u8 as FromStr>::Err;
    fn parse_arg(args: &mut Args) -> Result<Self, Err<Self::Err>> {
        let arg = args.pop()?;
        let res = u8::from_str(arg)?;
        Ok(res)
    }
}

impl ParseArg for String {
    type Err = Void;
    fn parse_arg(args: &mut Args) -> Result<Self, Err<Self::Err>> {
        args.pop().map(String::from)
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

#[derive(Debug)]
pub struct RealizedError {
    description: String,
    cause: Option<Box<Error>>
}

impl RealizedError {
    pub fn new<E: Error>(err: E) -> RealizedError {
        RealizedError {
            description: err.description().to_string(),
            cause: err.cause().map(???) // What goes inside the map?
        }
    }
}

impl Display for RealizedError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        writeln!(f, "{}", self.description)
    }
}

impl Error for RealizedError {
    fn description(&self) -> &str {
        self.description.as_ref()
    }

    fn cause(&self) -> Option<&Error> {
        // self.cause.map(|c| c.as_ref())
        None
    }
}

pub trait CoproductError: Sized {
    // fn coproduct_error(self) -> RealizedError;
}

// impl CoproductError for CNil {
//     fn coproduct_error(self) -> RealizedError {
//         match self { }
//     }
// }

// impl<H: Error, T: CoproductError> CoproductError for Coproduct<H, T> {
//     fn coproduct_error(self) -> RealizedError {
//         match self {
//             Coproduct::Inl(err)  => RealizedError::new(err),
//             Coproduct::Inr(next) => next.coproduct_error()
//         }
//     }
// }

pub trait Parse: HList {
    type Err;
    fn parse(args: Args) -> Result<Self, Err<Self::Err>>;
    fn parse_explain<F>(args: Args, f: F)
    where F: FnOnce(Self) -> (),
          Self::Err: CoproductError
    {
        match Self::parse(args) {
            Ok(parsed) => f(parsed),
            // Err(err) => eprintln!("{}", err.map(|c| c.coproduct_error()).description())
            Err(_) => panic!()
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
        let vec : Vec<&str> = vec![];
        let args = Args::new(&vec);
        assert_eq!(<Hlist![]>::parse(args), Ok(hlist![]));
    }

    #[test]
    fn parse_many() {
        let vec = vec!["foobar", "2", "not-a-number"];
        let args = Args::new(&vec);
        assert_eq!(
            <Hlist![String, u8, Option<u8>, String]>::parse(args),
            Ok(hlist!["foobar".to_string(), 2, None, "not-a-number".to_string()])
        );
    }
}
