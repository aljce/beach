extern crate itertools;

mod expr;
use expr::{Expr, Arg};

pub fn example() {
  let x = Expr::Command {
        name: "find",
        args: vec![Arg::Short("t"),Arg::Name("f"),Arg::Long("name"),Arg::Name("result")],
    };
    println!("{}", x);
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
