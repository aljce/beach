use std::process::{Command, Child, ExitStatus};
use std::cell::RefCell;
use std::path::PathBuf;
use std::env::current_dir;

pub mod expr;
pub use self::expr::*;

/// The mutable state that backs a shell (environment variables, current directory, ...)
pub struct Env {
    current_dir: RefCell<PathBuf>
}

impl Env {
    pub fn new() -> Env {
        let dir = current_dir().expect("Insufficient permissions");
        Env {
            current_dir: RefCell::new(dir)
        }
    }
}

pub enum Result<A> {
    Cd,
    Exit,
    Normal(A)
}

impl<A> Result<A> {
    /// Result is a Functor
    fn map<B,F>(self, f: F) -> Result<B> where F: Fn(A) -> B {
        use self::Result::*;
        match self {
            Cd => Cd,
            Exit => Exit,
            Normal(a) => Normal(f(a))
        }
    }
}

fn cd(env: &Env, path: &str) {
    let mut buf = env.current_dir.borrow_mut();
    let path = PathBuf::from(path);
    if path.is_dir() {
        if path.is_absolute() {
            *buf = path;
        } else {
            *buf = buf.join(path);
        }
    } else {
        println!("cd: requires the argument to be a directory and to be accessable")
    }

}

fn process(env: &Env, c: Process) -> Result<Child> {
    match c.name {
        // TODO: Look into how to use cd & exit programs so this hack isnt needed
        Program::Cd => {
            let number_args = c.args.len();
            if number_args != 1 {
                println!("cd: requires exactly 1 argument you gave [{}]", number_args)
            } else {
                cd(env, c.args[0])
            }
            Result::Cd
        },
        Program::Exit => Result::Exit,
        Program::Other(name) => {
            let mut p = Command::new(name);
            Result::Normal(
                p.args(c.args)
                 .current_dir(env.current_dir.borrow().clone())
                 .spawn()
                 .expect(&format!("Could not start {} process", name))
            )
        }
    }
}

fn sequence(_env: &Env, _left: Process, op: Operator, _right: Expr) -> ! {
    panic!("{} unimplemented", op)
}

fn redirect<'a>(_env: &Env, _expr: Expr<'a>, _file: &'a str) -> ! {
    panic!("redirect unimplemented")
}

fn expr(env: &Env, e: Expr) -> Result<Child> {
    match e {
        Expr::Base(c) => process(env, c),
        Expr::Sequence { left, op, right } => sequence(env, left, op, *right),
        Expr::Redirect { expr, file } => redirect(env, *expr, file)
    }
}

pub fn exec(env: &Env, e: Expr) -> Result<ExitStatus> {
    let p = expr(env, e);
    p.map(|mut child| child.wait().unwrap())
}
