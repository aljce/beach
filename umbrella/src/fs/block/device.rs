use std::str::{self, FromStr};
use std::path::{Path, PathBuf};
use std::fmt::{self, Debug, Display, Formatter};
use std::result;
use std::io::{self, Read, Write, Seek, SeekFrom};
use std::fs::{File, OpenOptions};
use nom::{Err, digit};

pub enum Error {
    Parse(Err),
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
            Error::Parse(ref err) => write!(f, "Parse error: {}", err),
            Error::IO(ref err)    => write!(f, "{}", err),
            Error::Size(ref err)  => write!(f, "{}", err)
        }
    }
}

impl From<Err> for Error {
    fn from(nom_err: Err) -> Error {
        Error::Parse(nom_err)
    }
}

impl From<io::Error> for Error {
    fn from(io_err: io::Error) -> Error {
        Error::IO(io_err)
    }
}

type Result<A> = result::Result<A, Error>;

#[derive(Clone, Debug, PartialEq)]
pub struct DevicePath<'a> {
    file:        &'a Path,
    block_size:  u16
}

impl<'a> Into<PathBuf> for DevicePath<'a> {
    fn into(self) -> PathBuf {
        PathBuf::from(format!("{}.{}.dev", self.file.to_string_lossy(), self.block_size))
    }
}

named!(
    device_path<DevicePath>,
    do_parse!(
        file: map!(
            take_till_s!(|c| c == b'.'),
            |bytes| Path::new(str::from_utf8(bytes).unwrap())
        ) >>
        char!('.') >>
        block_size: map_res!(
            map_res!(digit, str::from_utf8),
            FromStr::from_str
        ) >>
        tag!(".dev") >>
        (DevicePath { file, block_size })
    )
);

impl<'a> DevicePath<'a> {
    pub fn new(s: &'a str) -> Result<DevicePath<'a>> {
        let res = device_path(s.as_bytes()).to_result()?;
        Ok(res)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeviceConfig<'a> {
    path:        DevicePath<'a>,
    block_count: u64
}

impl<'a> DeviceConfig<'a> {
    pub fn new(s: &'a str) -> Result<DeviceConfig<'a>>  {
        let dev_path = DevicePath::new(s)?;
        Ok(DeviceConfig {
            path: dev_path,
            block_count: 0
        })
    }

    pub fn block_size(mut self, block_size: u16) -> Self {
        self.path.block_size = block_size;
        self
    }

    pub fn block_count(mut self, block_count: u64) -> Self {
        self.block_count = block_count;
        self
    }

    pub fn create(self) -> Result<BlockDevice<'a>> {
        if self.block_count <= 0 {
            let err_msg = format!(
                "create: block_count [{}] is less than 1",
                self.block_count
            );
            return Err(Error::Size(err_msg))
        }
        let file : PathBuf = self.path.clone().into();
        let mut handle = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(file)?;
        let seek_pos = SeekFrom::Start(self.path.block_size as u64 * self.block_count - 1);
        handle.seek(seek_pos)?;
        handle.write(&mut [0])?;
        Ok(BlockDevice { config: self, handle })
    }
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct BlockNumber {
    number: u64
}

impl BlockNumber {
    pub fn new(number: u64) -> BlockNumber {
        BlockNumber { number }
    }
}

impl Debug for BlockNumber {
    fn fmt(&self, f: &mut Formatter) -> result::Result<(), fmt::Error> {
        write!(f, "{}", self)
    }
}

impl Display for BlockNumber {
    fn fmt(&self, f: &mut Formatter) -> result::Result<(), fmt::Error> {
        write!(f, "{}", self.number)
    }
}

pub struct BlockDevice<'a> {
    config: DeviceConfig<'a>,
    handle: File
}

impl<'a> BlockDevice<'a> {
    pub fn create(file: &'a str, count: u64) -> Result<BlockDevice<'a>> {
        DeviceConfig::new(file)?.block_count(count).create()
    }

    pub fn open(file: &'a str) -> Result<BlockDevice<'a>> {
        let mut config = DeviceConfig::new(file)?;
        let handle = OpenOptions::new()
            .read(true)
            .write(true)
            .open(file)?;
        let file_len = handle.metadata()?.len();
        let block_size = config.path.block_size;
        config.block_count = file_len / block_size as u64;
        Ok(BlockDevice { config, handle })
    }

    fn seek(&mut self, block_num: BlockNumber, buf: &mut [u8]) -> Result<()> {
        let block_count = self.config.block_count;
        if block_count <= block_num.number {
            let err_msg = format!(
                "create: block_count [{}] is greater than the requested block number [{}]",
                block_count,
                block_num
            );
            return Err(Error::Size(err_msg))
        }
        let block_size = self.config.path.block_size;
        let buf_len = buf.len();
        if block_size as usize != buf_len {
            let err_msg = format!(
                "read: buffer length [{}] does not equal block_size [{}]",
                buf_len,
                block_size
            );
            return Err(Error::Size(err_msg))
        }
        let seek_pos = SeekFrom::Start(block_num.number * self.config.path.block_size as u64);
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
    fn block_device_deserialize() {
        let block_dev = DevicePath {
            file: Path::new("mydev"),
            block_size: 1024
        };
        parses_to(super::device_path("mydev.1024.dev".as_bytes()), block_dev)
    }

    #[test]
    fn block_device_serialize() {
        let block_dev : PathBuf = DevicePath {
            file: Path::new("mydev"),
            block_size: 1024
        }.into();
        assert_eq!(block_dev, PathBuf::from("mydev.1024.dev"))
    }
    #[test]

    fn block_write_read() {
        let mut block_device = BlockDevice::create("mydev.128.dev", 16).unwrap();
        let mut nums = (0..128).collect::<Vec<_>>();
        let mut out  = vec![0; 128];
        block_device.write(BlockNumber::new(1), nums.as_mut_slice()).unwrap();
        block_device.read(BlockNumber::new(1), out.as_mut_slice()).unwrap();
        assert_eq!(nums, out);
    }
}
