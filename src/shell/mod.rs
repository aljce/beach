use std::process;

pub mod expr;
pub use self::expr::*;

fn command(c: Command) {
    let p = process::Command::new(c.name)
        .args(c.args)
        .spawn();
}

fn sequence(_left: Command, op: Operator, _right: Expr) {
    panic!("{} unimplemented", op)
}

fn redirect<'a>(_expr: Expr<'a>, _file: &'a str) {
    panic!("redirect unimplemented")
}

pub fn exec(e: Expr) {
    match e {
        Expr::Base(c) => command(c),
        Expr::Sequence { left, op, right } => sequence(left, op, *right),
        Expr::Redirect { expr, file } => redirect(*expr, file)
    }
}
