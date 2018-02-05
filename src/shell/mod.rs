use std::result;
use std::io;
use std::process::{Command, Stdio, ExitStatus};
use std::cell::RefCell;
use std::path::PathBuf;
use std::env::current_dir;
use std::fs::File;

pub mod expr;
pub use self::expr::*;

/// The mutable state that backs a shell (environment variables, current directory, ...)
pub struct Env {
    current_dir: RefCell<PathBuf>
}

impl Env {
    pub fn new() -> Env {
        let dir = current_dir().expect("ERROR: Insufficient permissions to read master process current directory");
        Env {
            current_dir: RefCell::new(dir)
        }
    }
}

fn cd(env: &Env, args: Vec<&str>) {
    // TODO: Look into how to use cd & exit programs so this hack isnt needed
    let number_args = args.len();
    if number_args != 1 {
        eprintln!("ERROR: cd requires exactly 1 argument you gave [{}]", number_args)
    } else {
        let path = PathBuf::from(args[0]);
        if path.is_dir() {
            let mut buf = env.current_dir.borrow_mut();
            if path.is_absolute() {
                *buf = path;
            } else {
                *buf = buf.join(path);
            }
        } else {
            eprintln!("ERROR: cd requires the argument to be a directory and to be accessable")
        }
    }
}

pub enum ProcessErr {
    Continue,
    Exit,
    Error(io::Error),
    Pipe
}

impl From<io::Error> for ProcessErr {
    fn from(err: io::Error) -> ProcessErr {
        ProcessErr::Error(err)
    }
}

type ProcessResult<A> = result::Result<A, ProcessErr>;

fn process(env: &Env, c: Process) -> ProcessResult<Command> {
    match c.name {
        Program::Cd => {
            cd(env, c.args);
            Err(ProcessErr::Continue)
        }
        Program::Exit => Err(ProcessErr::Exit),
        Program::Other(name) => {
            let mut command = Command::new(name);
            command
                .args(c.args)
                .current_dir(env.current_dir.borrow().clone());
            Ok(command)
        }
    }
}

fn sequence(env: &Env, left: Process, op: Operator, right: Expr) -> ProcessResult<Command> {
    let mut left_command = process(env, left)?;
    let mut right_command = expr(env, right)?;
    match op {
        Operator::Pipe => {
            let mut left_child = left_command.stdout(Stdio::piped()).spawn()?;
            let left_stdout = match left_child.stdout {
                None => return Err(ProcessErr::Pipe),
                Some(stdin) => stdin
            };
            right_command.stdin(left_stdout);
            Ok(right_command)
        }
        Operator::Or => {
            let mut left_child = left_command.spawn()?;
            let left_exit = left_child.wait()?;
            if left_exit.success() {
                Ok(left_command)
            } else {
                Ok(right_command)
            }
        }
        Operator::And => {
            let mut left_child = left_command.spawn()?;
            let left_exit = left_child.wait()?;
            if left_exit.success() {
                Ok(right_command)
            } else {
                Ok(left_command)
            }
        }
    }
}

fn redirect<'a>(env: &Env, e: Expr<'a>, file: &'a str) -> ProcessResult<Command> {
    let mut command = expr(env, e)?;
    let file = File::create(file)?;
    command.stdout(file);
    Ok(command)
}

fn expr(env: &Env, e: Expr) -> ProcessResult<Command> {
    match e {
        Expr::Base(c) => process(env, c),
        Expr::Sequence { left, op, right } => sequence(env, left, op, *right),
        Expr::Redirect { expr, file } => redirect(env, *expr, file)
    }
}

pub fn exec(env: &Env, e: Expr) -> ProcessResult<ExitStatus> {
    let mut command = expr(env, e)?;
    let mut child = command.spawn()?;
    let exit_code = child.wait()?;
    Ok(exit_code)
}
