use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::env::current_dir;

use umbrella::block::device::{BlockNumber, BlockDevice};
use umbrella::block::{FileSystem};

pub mod parse;
pub use self::parse::*;

/// The mutable state that backs a shell (environment variables, current directory, ...)
pub struct Env {
    current_dir: RefCell<PathBuf>,
    current_fs:  RefCell<Option<FileSystem>>
}

impl Env {
    pub fn new() -> Env {
        let dir = current_dir().expect("ERROR: Insufficient permissions to read master process current directory");
        Env {
            current_dir: RefCell::new(dir),
            current_fs:  RefCell::new(None)
        }
    }

    pub fn current_dir(&self) -> PathBuf {
        self.current_dir.borrow().clone()
    }

    pub fn with_fs<F>(&self, f: F)
        where F: FnOnce(&FileSystem) -> ()
    {
        match *self.current_fs.borrow() {
            Some(ref fs) => f(fs),
            None => eprintln!("ERROR: No file system mounted, try running newfs then mount")
        }
    }
}

pub fn cd(env: &Env, args: Args) {
    let parser = Parser::new(vec![Argument::string()]);
    args.parse_explain("cd", parser, |parsed| {
        let path = PathBuf::from(parsed.at(0).string());
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
    })
}

pub fn new_fs(_env: &Env, args: Args) {
    let parser = Parser::new(
        vec![ Argument::string()
            , Argument::nat()
            , Argument::Optional(Kind::Nat)
        ]
    );
    args.parse_explain("newfs", parser, |parsed| {
        let file_name = parsed.at(0).string();
        let block_count = parsed.at(1).nat();
        let block_size = parsed.optional(2).nat().map(|n| n as u16);
        match BlockDevice::create(&file_name, block_count, block_size) {
            Ok(mut device) => {
                if device.config.block_size < 128 {
                    eprintln!(
                        "ERROR: The block size must be at least 128 you gave: {}",
                        device.config.block_size
                    );
                    return
                }
                let newfs = FileSystem::new(&device);
                newfs.write(&mut device).unwrap_or_else(|err| {
                    eprintln!("ERROR: Could not initialize file system: {}", err);
                });
            }
            Err(err) => eprintln!("ERROR: {}", err)
        }
    })
}

pub fn mount(env: &Env, args: Args) {
    let parser = Parser::new(vec![Argument::string()]);
    args.parse_explain("mount", parser, |parsed| {
        let file_name = parsed.at(0).string();
        if ! Path::new(&file_name).exists() {
            eprintln!(
                "ERROR: The device: {0} does not exist. Try running 'newfs {0} 128' first.",
                file_name
            );
            return
        }
        match BlockDevice::open(&file_name) {
            Ok(device) => {
                match FileSystem::read(device) {
                    Ok(fs) => {
                        // fs.write_sync_status(&mut device, true);
                        let mut cur_fs = env.current_fs.borrow_mut();
                        *cur_fs = Some(fs);
                    }
                    Err(err) => {
                        eprintln!("ERROR: Could not sync filesystem because {}", err)
                    }
                }
            }
            Err(err) => eprintln!("ERROR: {}", err)
        }
    })
}

pub fn block_map(env: &Env, _args: Args) {
    env.with_fs(|fs| {
        print!("{}", fs.block_map)
    })
}

pub fn alloc_block(_env: &Env, _args: Args) {
    eprintln!("unimplemented");
}

pub fn free_block(_env: &Env, args: Args) {
    let parser = Parser::new(vec![Argument::nat()]);
    args.parse_explain("free_block", parser, |parsed| {
        let _block_number = BlockNumber::new(parsed.at(0).nat());
        eprintln!("unimplemented");
    })
}

pub fn inode_map(_env: &Env, _args: Args) {
    eprintln!("unimplemented");
}

pub fn alloc_inode(_env: &Env, args: Args) {
    let parser = Parser::new(vec![Argument::string()]);
    args.parse_explain("alloc_inode", parser, |parsed| {
        let _inode_type = parsed.at(0).string();
        eprintln!("unimplemented");
    })
}

pub fn free_inode(_env: &Env, args: Args) {
    let parser = Parser::new(vec![Argument::nat()]);
    args.parse_explain("free_inode", parser, |parsed| {
        let _block_number = BlockNumber::new(parsed.at(0).nat());
        eprintln!("unimplemented");
    })

}

pub fn unmount(_env: &Env, _args: Args) {
    eprintln!("unimplemented");
}


