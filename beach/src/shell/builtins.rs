use std::cell::RefCell;
use std::path::PathBuf;
use std::env::current_dir;

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

    pub fn current_dir(&self) -> PathBuf {
        self.current_dir.borrow().clone()
    }
}

pub fn cd(env: &Env, args: Vec<String>) {
    let number_args = args.len();
    if number_args != 1 {
        eprintln!("ERROR: cd requires exactly 1 argument you gave [{}]", number_args)
    } else {
        let path = PathBuf::from(&args[0]);
        if path.is_dir() {
            let mut buf = env.current_dir.borrow_mut();
            if path.is_absolute() {
                *buf = path;
            } else {
                *buf = buf.join(path);
            }
        } else {
            eprintln!("ERROR: cd requires the argument to be a directory and to be accessible")
        }
    }
}

pub fn new_fs(_env: &Env, _args: Vec<String>) {
    eprintln!("unimplemented");
}

pub fn mount(_env: &Env, _args: Vec<String>) {
    eprintln!("unimplemented");
}

pub fn block_map(_env: &Env, _args: Vec<String>) {
    eprintln!("unimplemented");
}

pub fn alloc_block(_env: &Env, _args: Vec<String>) {
    eprintln!("unimplemented");
}

pub fn free_block(_env: &Env, _args: Vec<String>) {
    eprintln!("unimplemented");
}

pub fn inode_map(_env: &Env, _args: Vec<String>) {
    eprintln!("unimplemented");
}

pub fn alloc_inode(_env: &Env, _args: Vec<String>) {
    eprintln!("unimplemented");
}

pub fn free_inode(_env: &Env, _args: Vec<String>) {
    eprintln!("unimplemented");
}

pub fn unmount(_env: &Env, _args: Vec<String>) {
    eprintln!("unimplemented");
}


