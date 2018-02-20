use std::result;
use std::io;
use std::process::{Command, Stdio, ExitStatus};
use std::fs::File;

pub mod expr;
pub mod builtins;
pub use self::expr::*;
pub use self::builtins::*;

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

type Result<A> = result::Result<A, ProcessErr>;

fn process(env: &Env, c: Process) -> Result<Command> {
    let prog = match c.name {
        Program::Cd => cd,
        Program::NewFS => new_fs,
        Program::Mount => mount,
        Program::BlockMap => block_map,
        Program::AllocBlock => alloc_block,
        Program::FreeBlock => free_block,
        Program::INodeMap => inode_map,
        Program::AllocINode => alloc_inode,
        Program::FreeINode => free_inode,
        Program::Unmount => unmount,
        Program::Exit => {
            return Err(ProcessErr::Exit)
        }
        Program::Other(name) => {
            let mut command = Command::new(name);
            command
                .args(c.args)
                .current_dir(env.current_dir());
            return Ok(command)
        }
    };
    prog(env, c.args);
    Err(ProcessErr::Continue)
}

fn sequence(env: &Env, left: Process, op: Operator, right: Expr) -> Result<Command> {
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

fn redirect(env: &Env, e: Expr, file: String) -> Result<Command> {
    let mut command = expr(env, e)?;
    let file = File::create(file)?;
    command.stdout(file);
    Ok(command)
}

fn expr(env: &Env, e: Expr) -> Result<Command> {
    match e {
        Expr::Base(c) => process(env, c),
        Expr::Sequence { left, op, right } => sequence(env, left, op, *right),
        Expr::Redirect { expr, file } => redirect(env, *expr, file)
    }
}

pub fn exec(env: &Env, e: Expr) -> Result<ExitStatus> {
    let mut command = expr(env, e)?;
    let mut child = command.spawn()?;
    let exit_code = child.wait()?;
    Ok(exit_code)
}
