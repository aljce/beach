use std::io::{self, Write};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::env::current_dir;

use umbrella::block::device::{BlockNumber, BlockDevice};
use umbrella::block::{FileSystem, Mount};

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

    const NO_MOUNT_MSG : &'static str = "ERROR: No file system mounted, try running newfs then mount";

    pub fn with_fs<F>(&self, f: F)
    where F: FnOnce(&mut FileSystem) -> ()
    {
        match *self.current_fs.borrow_mut() {
            Some(ref mut fs) => f(fs),
            None => eprintln!("{}", Env::NO_MOUNT_MSG)
        }
    }

    pub fn take_fs<F>(&self, f: F)
    where F: FnOnce(FileSystem) -> ()
    {
        let cur_fs = self.current_fs.replace(None);
        match cur_fs {
            Some(fs) => f(fs),
            None => eprintln!("{}", Env::NO_MOUNT_MSG)
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
            Ok(device) => {
                if device.config.block_size < 128 {
                    eprintln!(
                        "ERROR: The block size must be at least 128 you gave: {}",
                        device.config.block_size
                    );
                    return
                }
                let newfs = FileSystem::new(device);
                newfs.close().unwrap_or_else(|err| {
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
                "ERROR: The device {0} does not exist. Try running 'newfs {0} 128' first.",
                file_name
            );
            return
        }
        match BlockDevice::open(&file_name) {
            Ok(device) => {
                match FileSystem::read(device) {
                    Ok(Mount { clean_mount, file_system }) => {
                        if ! clean_mount {
                            eprintln!("WARNING: The filesystem was not properly unmounted")
                        }
                        let mut cur_fs = env.current_fs.borrow_mut();
                        *cur_fs = Some(file_system);
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
        print!("{}", fs.block_map);
        io::stdout().flush().unwrap()
    })
}

pub fn alloc_block(env: &Env, _args: Args) {
    env.with_fs(|fs| {
        match fs.block_map.alloc() {
            Some(block_number) => println!("alloc [{}]", block_number),
            None => println!("ERROR: No room left on device")
        }
    })
}

pub fn free_block(env: &Env, args: Args) {
    let parser = Parser::new(vec![Argument::nat()]);
    args.parse_explain("free_block", parser, |parsed| {
        let block_number = BlockNumber::new(parsed.at(0).nat());
        env.with_fs(|fs| {
            fs.block_map.free(block_number)
        })
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

pub fn unmount(env: &Env, _args: Args) {
    env.take_fs(|fs| {
        fs.close().unwrap_or_else(|err| {
            eprintln!("ERROR: File system was not unmounted cleanly because: {}", err)
        })
    })
}


