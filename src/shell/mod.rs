use std::process::{Command, Child, Stdio, ExitStatus};
use std::cell::RefCell;
use std::path::PathBuf;
use std::env::current_dir;
use std::io::Write;
use std::fs::File;

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

pub struct Pipes {
    stdin:  Stdio,
    stdout: Stdio
}

impl Pipes {
    pub fn inherit() -> Pipes {
        Pipes {
            stdin:  Stdio::inherit(),
            stdout: Stdio::inherit()
        }
    }

    pub fn piped_stdout(stdin: Stdio) -> Pipes {
        Pipes {
            stdin:  stdin,
            stdout: Stdio::piped()
        }
    }

    pub fn piped_stdin(stdout: Stdio) -> Pipes {
        Pipes {
            stdin:  Stdio::piped(),
            stdout: stdout
        }
    }
}


pub enum Result<A> {
    Continue,
    Exit,
    Normal(A)
}

impl<A> Result<A> {

    /// Result is a Functor
    fn map<B,F>(self, f: F) -> Result<B> where F: FnOnce(A) -> B {
        use self::Result::*;
        self.flat_map(|a| Normal(f(a)))
    }

    // Result is a Monad
    fn flat_map<B,F>(self, f: F) -> Result<B> where F: FnOnce(A) -> Result<B> {
        use self::Result::*;
        match self {
            Continue => Continue,
            Exit => Exit,
            Normal(a) => f(a)
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

fn process(env: &Env, pipes: Pipes, c: Process) -> Result<Child> {
    match c.name {
        // TODO: Look into how to use cd & exit programs so this hack isnt needed
        Program::Cd => {
            let number_args = c.args.len();
            if number_args != 1 {
                println!("cd: requires exactly 1 argument you gave [{}]", number_args)
            } else {
                cd(env, c.args[0])
            }
            Result::Continue
        }
        Program::Exit => Result::Exit,
        Program::Other(name) => {
            let mut p = Command::new(name);
            Result::Normal(
                p.args(c.args)
                 .current_dir(env.current_dir.borrow().clone())
                 .stdin(pipes.stdin)
                 .stdout(pipes.stdout)
                 .spawn()
                 .expect(&format!("Could not start {} process", name))
            )
        }
    }
}

fn sequence(env: &Env, pipes: Pipes, left: Process, op: Operator, right: Expr) -> Result<Child> {
    let stdout = Pipes::piped_stdout(pipes.stdin);
    let stdin  = Pipes::piped_stdin(pipes.stdout);
    process(env, stdout, left).flat_map(|left_child| {
        let output = left_child.wait_with_output().expect("Could not read stdout from child process");
        match op {
            Operator::Pipe => {
                /*
                expr(env, stdin, right).map(|right_child| {
                    let mut child_stdin = right_child.stdin.expect("Could not read stdin from child process");
                    child_stdin.write_all(&output.stdout).expect("Could not aquire stdin lock for child process");
                    right_child
                })
                */
                unimplemented!()
            }
            Operator::Or => {
                unimplemented!()
            }
            Operator::And => {
                unimplemented!()
            }
        }

    })
}

fn redirect<'a>(env: &Env, pipes: Pipes, e: Expr<'a>, file: &'a str) -> Result<Child> {
    let piped = Pipes::piped_stdout(pipes.stdin);
    expr(env, piped, e).flat_map(|child| {
        let output = child.wait_with_output().expect("Could not read stdout from child process");
        let mut file = File::create(file).unwrap();
        file.write_all(&output.stdout).unwrap();
        Result::Continue
    })
}

fn expr(env: &Env, pipes: Pipes, e: Expr) -> Result<Child> {
    match e {
        Expr::Base(c) => process(env, pipes, c),
        Expr::Sequence { left, op, right } => sequence(env, pipes, left, op, *right),
        Expr::Redirect { expr, file } => redirect(env, pipes, *expr, file)
    }
}

pub fn exec(env: &Env, e: Expr) -> Result<ExitStatus> {
    let p = expr(env, Pipes::inherit(), e);
    p.map(|mut child| child.wait().unwrap())
}
