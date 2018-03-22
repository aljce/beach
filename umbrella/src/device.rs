use std::str::{self, FromStr};
use std::path::PathBuf;
use std::fmt::{self, Debug, Display, Formatter};
use std::result;
use std::io::{self, Read, Write, Seek, SeekFrom};
use std::fs::{File, OpenOptions};
use nom::{Err, digit};
use bincode;

use block_number::{BlockNumber};

pub enum Error {
    Parse(Err),
    Bincode(Box<bincode::ErrorKind>),
    IO(io::Error),
    Size(String)
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter) -> result::Result<(), fmt::Error> {
        write!(f, "{}", self)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> result::Result<(), fmt::Error> {
        match *self {
            Error::Parse(ref err)   => write!(f, "Parse error: {}", err),
            Error::Bincode(ref err) => write!(f, "(de)serialization error: {}", err),
            Error::IO(ref err)      => write!(f, "{}", err),
            Error::Size(ref err)    => write!(f, "{}", err)
        }
    }
}

impl From<Err> for Error {
    fn from(nom_err: Err) -> Error {
        Error::Parse(nom_err)
    }
}

impl From<Box<bincode::ErrorKind>> for Error {
    fn from(bincode_err: Box<bincode::ErrorKind>) -> Error {
        Error::Bincode(bincode_err)
    }
}

impl From<io::Error> for Error {
    fn from(io_err: io::Error) -> Error {
        Error::IO(io_err)
    }
}

pub type Result<A> = result::Result<A, Error>;

named!(
    file_components<(PathBuf, u16)>,
    do_parse!(
        file: map!(
            take_till_s!(|c| c == b'.'),
            |bytes| PathBuf::from(str::from_utf8(bytes).unwrap())
        ) >>
        char!('.') >>
        block_size: map_res!(
            map_res!(digit, str::from_utf8),
            FromStr::from_str
        ) >>
        tag!(".dev") >>
        ((file, block_size))
    )
);

#[derive(Clone, Debug, PartialEq)]
pub struct DeviceConfig {
        path:        PathBuf,
    pub block_size:  u16,
    pub block_count: u64
}

impl DeviceConfig {
    pub fn new(path: &str) -> DeviceConfig  {
        DeviceConfig {
            path:        PathBuf::from(path),
            block_size:  0,
            block_count: 0
        }
    }

    pub fn parse(s: &str) -> Result<DeviceConfig> {
        let (path, block_size) = file_components(s.as_bytes()).to_result()?;
        Ok(DeviceConfig { path, block_size, block_count: 0 })
    }

    pub fn block_size(mut self, block_size: u16) -> Self {
        self.block_size = block_size;
        self
    }

    pub fn block_count(mut self, block_count: u64) -> Self {
        self.block_count = block_count;
        self
    }

    pub fn file(&self) -> PathBuf {
        PathBuf::from(format!("{}.{}.dev", self.path.to_string_lossy(), self.block_size))
    }
}

pub struct BlockDevice {
    pub config: DeviceConfig,
        handle: File
}

impl BlockDevice {
    pub fn create(path: &str, count: u64, optional_size: Option<u16>) -> Result<BlockDevice>
    {
        let size = optional_size.unwrap_or(1024);
        let config = DeviceConfig::new(path)
            .block_count(count)
            .block_size(size);
        if config.block_count <= 0 {
            let err_msg = format!(
                "create: block_count [{}] is less than 1",
                config.block_count
            );
            return Err(Error::Size(err_msg))
        }
        if config.block_size <= 0 {
            let err_msg = format!(
                "create: block_size [{}] is less than 1",
                config.block_size
            );
            return Err(Error::Size(err_msg))
        }
        let file = config.file();
        let mut handle = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(file)?;
        let seek_pos = SeekFrom::Start(config.block_size as u64 * config.block_count - 1);
        handle.seek(seek_pos)?;
        handle.write(&mut [0])?;
        Ok(BlockDevice { config, handle })
    }


    pub fn open(path: &str) -> Result<BlockDevice> {
        let mut config = DeviceConfig::parse(path)?;
        let handle = OpenOptions::new()
            .read(true)
            .write(true)
            .open(config.file())?;
        let file_len = handle.metadata()?.len();
        let block_size = config.block_size;
        config.block_count = file_len / block_size as u64;
        Ok(BlockDevice { config, handle })
    }

    fn seek(&mut self, block_num: BlockNumber, buf: &mut [u8]) -> Result<()> {
        let block_count = self.config.block_count;
        if block_count <= block_num.number {
            let err_msg = format!(
                "create: block_count [{}] is less than the requested block number [{}]",
                block_count,
                block_num
            );
            return Err(Error::Size(err_msg))
        }
        let block_size = self.config.block_size;
        let buf_len = buf.len();
        if block_size as usize != buf_len {
            let err_msg = format!(
                "read: buffer length [{}] does not equal block_size [{}]",
                buf_len,
                block_size
            );
            return Err(Error::Size(err_msg))
        }
        let seek_pos = SeekFrom::Start(block_num.number * self.config.block_size as u64);
        self.handle.seek(seek_pos)?;
        Ok(())
    }

    pub fn read(&mut self, block_num: BlockNumber, buf: &mut [u8]) -> Result<()> {
        self.seek(block_num, buf)?;
        self.handle.read_exact(buf)?;
        Ok(())
    }

    pub fn write(&mut self, block_num: BlockNumber, buf: &mut [u8]) -> Result<()> {
        self.seek(block_num, buf)?;
        self.handle.write_all(buf)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::fmt::{Debug};
    use nom::{IResult};

    use super::*;

    fn parses_to<A>(res: IResult<&[u8], A>, correct: A)
    where A: PartialEq<A> + Debug
    {
        match res {
            IResult::Done(i,o) => {
                if o != correct {
                    let i_str = str::from_utf8(i).unwrap();
                    panic!(
                        "{:?} does not equal {:?} and the parse had these leftovers: {}",
                        o, correct, i_str
                    )
                }
            },
            IResult::Error(err) => panic!("error: {:?}", err),
            IResult::Incomplete(needed) => panic!("needed: {:?}", needed)
        }
    }

    #[test]
    fn file_components_deserialize() {
        let device_path = (PathBuf::from("mydev"), 1024);
        parses_to(super::file_components("mydev.1024.dev".as_bytes()), device_path)
    }

    #[test]
    fn file_components_serialize() {
        let file = DeviceConfig::new("mydev").block_size(1024).file();
        assert_eq!(file, PathBuf::from("mydev.1024.dev"))
    }

    #[test]
    fn block_write_read() {
        let mut block_device = BlockDevice::create("mydev", 16, Some(128)).unwrap();
        let mut nums = (0..128).collect::<Vec<_>>();
        let mut out  = vec![0; 128];
        block_device.write(BlockNumber::new(1), nums.as_mut_slice()).unwrap();
        block_device.read(BlockNumber::new(1), out.as_mut_slice()).unwrap();
        assert_eq!(nums, out);
    }
}
