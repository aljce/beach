use std::str::FromStr;
use std::num::ParseIntError;

// TODO: This whole module is a huge hack. This should be implemented like optparse-applicative
// or with hlists

#[derive(Clone, PartialEq)]
pub enum Kind {
    Nat,
    String
}

impl Kind {
    fn parse(&self, s: String) -> Result<Type, Err> {
        match *self {
            Kind::Nat => {
                let res = Type::Nat(u64::from_str(&s).map_err(|int_err| {
                    Err::Nat { int_err, failed: s }
                })?);
                Ok(res)
            }
            Kind::String => Ok(Type::String(s))
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum Type {
    Nat(u64),
    String(String)
}

impl Type {
    pub fn nat(&self) -> u64 {
        match *self {
            Type::Nat(n) => n,
            _ => panic!("argument is not a nat")
        }
    }

    pub fn string(&self) -> String {
        match *self {
            Type::String(ref s) => s.clone(),
            _ => panic!("argument is not a string")
        }
    }
}



#[derive(Clone, PartialEq)]
pub enum Argument {
    Required(Kind),
    Optional(Kind)
}

impl Argument {
    pub fn nat() -> Argument {
        Argument::Required(Kind::Nat)
    }

    pub fn string() -> Argument {
        Argument::Required(Kind::String)
    }
}

#[derive(Clone, PartialEq)]
pub enum Err {
    Nat {
        int_err: ParseIntError,
        failed:  String
    },
    MissingArguments,
}

impl Err {
    pub fn render(&self, prog: &str) -> String {
        match *self {
            Err::Nat { ref failed, .. } => {
                format!("ERROR: {} needs to be a nonnegative integer", failed)
            }
            Err::MissingArguments => {
                format!("ERROR: {} needs more arguments", prog)
            }
        }
    }

    pub fn explain(&self, prog: &str) {
        eprintln!("{}", self.render(prog))
    }
}

#[derive(Clone, PartialEq)]
pub struct Optional {
    option: Option<Type>
}

impl Optional {
    pub fn nat(self) -> Option<u64> {
        self.option.map(|t| t.nat())
    }
}

#[derive(Clone, PartialEq)]
pub enum ParsedArgument {
    Required(Type),
    Optional(Optional)
}

#[derive(Clone, PartialEq)]
pub struct Parsed {
    types: Vec<ParsedArgument>
}

impl Parsed {
    pub fn at(&self, i: usize) -> Type {
        match self.types[i].clone() {
            ParsedArgument::Required(t) => t,
            _ => panic!("optional argument required")
        }
    }

    pub fn optional(&self, i: usize) -> Optional {
        match self.types[i].clone() {
            ParsedArgument::Optional(t) => t,
            _ => panic!("required argument optional")
        }
    }
}

pub struct Parser {
    kinds: Vec<Argument>
}

impl Parser {
    pub fn new(kinds: Vec<Argument>) -> Parser {
        Parser { kinds }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Args {
    pub vec: Vec<String>
}

impl Args {
    pub fn parse(&self, parser: Parser) -> Result<Parsed, Err> {
        let mut types : Vec<ParsedArgument> = vec![];
        let len = self.vec.len();
        let mut i = 0;
        for kind in parser.kinds.iter() {
            match *kind {
                Argument::Required(ref kind) => {
                    if len <= i {
                        return Err(Err::MissingArguments)
                    }
                    let arg = self.vec[i].clone();
                    i += 1;
                    let t = kind.parse(arg)?;
                    types.push(ParsedArgument::Required(t));
                }
                Argument::Optional(ref kind) => {
                    if len <= i {
                        let none = Optional { option: None };
                        types.push(ParsedArgument::Optional(none));
                        continue
                    }
                    let arg = self.vec[i].clone();
                    let opt = match kind.parse(arg) {
                        Ok(t) => {
                            i += 1;
                            Optional { option: Some(t) }
                        }
                        Err(_) => Optional { option: None }
                    };
                    types.push(ParsedArgument::Optional(opt))
                }
            }
        }
        Ok(Parsed { types })
    }

    pub fn parse_explain<F>(&self, prog: &str, parser: Parser, with_parsed: F)
    where F: FnOnce(Parsed) -> () {
        match self.parse(parser) {
            Ok(parsed) => with_parsed(parsed),
            Err(err) => err.explain(prog)
        }
    }
}
