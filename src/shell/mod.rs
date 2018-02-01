pub mod expr;
pub use self::expr::*;

fn command(_c: Command) {
}

fn sequence(_left: Command, _op: Operator, _right: Expr) {
}

fn redirect<'a>(_expr: Expr<'a>, _file: &'a str) {
}

pub fn exec(e: Expr) {
    match e {
        Expr::Base(c) => command(c),
        Expr::Sequence { left, op, right } => sequence(left, op, *right),
        Expr::Redirect { expr, file } => redirect(*expr, file)
    }
}
