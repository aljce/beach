use std::process::{Command, Child, ExitStatus};
use std::path::PathBuf;
use std::env::current_dir;

pub mod expr;
pub use self::expr::*;

/// The mutable state that backs a shell (environment variables, current directory, ...)
pub struct Env {
    current_dir: PathBuf
}

impl Env {
    pub fn new() -> Env {
        Env {
            current_dir: current_dir().expect("Insufficient permissions")
        }
    }
}

fn process(env: Env, c: Process) -> Child {
    let mut p = Command::new(c.name);
    p.args(c.args)
     .current_dir(env.current_dir)
     .spawn()
     .expect(&format!("Could not start {} process", c.name))
}

fn sequence(_env: Env, _left: Process, op: Operator, _right: Expr) -> ! {
    panic!("{} unimplemented", op)
}

fn redirect<'a>(_env: Env, _expr: Expr<'a>, _file: &'a str) -> ! {
    panic!("redirect unimplemented")
}

fn expr(env: Env, e: Expr) -> Child {
    match e {
        Expr::Base(c) => process(env, c),
        Expr::Sequence { left, op, right } => sequence(env, left, op, *right),
        Expr::Redirect { expr, file } => redirect(env, *expr, file)
    }
}

pub fn exec(e: Expr) -> ExitStatus {
    let mut p = expr(Env::new(), e);
    p.wait().unwrap()
}
