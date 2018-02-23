use std::fmt::{self, Display, Formatter};
use std::time::{SystemTime};
use bit_vec::BitVec;
use bincode::{serialize_into, deserialize_from};

pub mod device;
use self::device::{BlockNumber, BlockDevice, MASTER_BLOCK_NUMBER};

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct MasterBlockFlags: u8 {
        const SYNCED = 0b10000000;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MasterBlock {
    block_size:  u16,
    block_count: u64,
    inode_count: u16,
    block_map:   BlockNumber,
    inode_map:   BlockNumber,
    pub flags:   MasterBlockFlags,
}

impl MasterBlock {
    pub fn new(block_size: u16, block_count: u64, inode_count: u16) -> MasterBlock {
        MasterBlock {
            block_size,
            block_count,
            inode_count,
            block_map: BlockNumber::new(2),
            inode_map: BlockNumber::new(3 + (block_count / block_size as u64) / 8),
            flags:     MasterBlockFlags::SYNCED
        }
    }

    pub fn block_map_blocks(&self) -> usize {
        ((self.block_count / self.block_size as u64) / 8) as usize
    }

    pub fn write(&self, device: &mut BlockDevice) -> device::Result<()> {
        let mut mb_vec = vec![0; self.block_size as usize];
        serialize_into(&mut mb_vec[..], &self)?;
        device.write(MASTER_BLOCK_NUMBER, &mut mb_vec[..])
    }

    pub fn write_sync_status(&mut self, device: &mut BlockDevice, status: bool) -> device::Result<()> {
        let mut master_block = self.clone();
        master_block.flags.set(MasterBlockFlags::SYNCED, status);
        master_block.write(device)?;
        *self = master_block;
        Ok(())
    }
}

// Theoretically this should be a doubly linked free list as the asymptotics on all the operations
// I want to support would be optimal. I choose a bitvec even though its asymptotics are worse. I
// did this because bitvecs have much lower constants on all the operations in question.
pub struct BlockMap {
    vec:  BitVec,
    free: Option<BlockNumber>
}

impl BlockMap {
    pub fn new(block_count: u64) -> BlockMap {
        BlockMap {
            vec: BitVec::from_elem(block_count as usize, false),
            free: Some(BlockNumber::new(0))
        }
    }

    fn find_free(vec: &BitVec) -> Option<usize> {
        vec.iter().enumerate().find(|&(_, b)| b == false).map(|(i, _)| i)
    }

    pub fn init(vec: BitVec) -> BlockMap {
        let free = BlockMap::find_free(&vec);
        BlockMap { vec, free: free.map(|i| BlockNumber::new(i as u64)) }
    }

    pub fn alloc(&mut self) -> Option<BlockNumber> {
        BlockMap::find_free(&self.vec).map(|i| {
            self.vec.set(i, true);
            BlockNumber::new(i as u64)
        })
    }

    pub fn free(&mut self, block_number: BlockNumber) {
        self.vec.set(block_number.index(), false)
    }
}

fn display_chunks<I, F>(items: I, display: F, f: &mut Formatter) -> Result<(), fmt::Error>
where I: Iterator, F: Fn(I::Item) -> char
{
    let mut s = String::new();
    let mut i = 0;
    let mut flushed = true;
    for item in items {
        s.push(display(item));
        i += 1;
        if i != 64 {
            flushed = false;
            if i % 8 == 0 {
                s.push('|');
            }
        } else {
            writeln!(f, "{}", s)?;
            s = String::new();
            i = 0;
            flushed = true;
        }
    }
    if ! flushed {
        writeln!(f, "{}", s)?
    }
    Ok(())

}

impl Display for BlockMap {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        fn display(b: bool) -> char {
            if b {'1'} else {'0'}
        }
        display_chunks(self.vec.iter(), display, f)
    }
}


bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct INodeFlags: u8 {
        const FREE = 0b1000_0000;
        const FILE = 0b0100_0000;
        const DIR  = 0b0010_0000;
        const LINK = 0b0001_0000;
        const PTR  = 0b0000_1000;
        const DATA = 0b0000_0100;
    }
}


named!(
    parse_inode_flags<INodeFlags>,
    alt!(
        value!(INodeFlags::FREE, char!('0')) |
        value!(INodeFlags::FILE, char!('f')) |
        value!(INodeFlags::LINK, char!('s')) |
        value!(INodeFlags::PTR,  char!('d')) |
        value!(INodeFlags::DATA, char!('D'))
    )
);

impl INodeFlags {
    pub fn parse(s: &str) -> device::Result<INodeFlags> {
        let res = parse_inode_flags(s.as_bytes()).to_result()?;
        Ok(res)
    }
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    struct Permissions: u16 {
        const UNUSED = 0b0000_0000_0000_0000;
    }
}
// I use rusts SystemTime to represent and serialize time. This type cannot be fit into
// 32 bits but that restriction is silly and wrong. See the 2038 unix-time apocalypse for details.
#[derive(Serialize, Deserialize)]
pub struct INode {
    inode_num:  u16,
    cdate:      SystemTime,
    mdate:      SystemTime,
    flags:      INodeFlags,
    perms:      Permissions,
    level:      u8,
    block_ptrs: [BlockNumber; 12]
}

impl INode {
    pub fn new(inode_num: u16, now: SystemTime) -> INode {
        INode {
            inode_num,
            cdate: now,
            mdate: now,
            flags: INodeFlags::FREE,
            perms: Permissions::UNUSED,
            level: 0,
            block_ptrs: [BlockNumber::new(1); 12]
        }
    }
}

pub struct INodeMap {
    vec: Vec<INode>
}

impl INodeMap {
    pub fn new(inode_count: u16) -> INodeMap {
        let now = SystemTime::now();
        let nodes = (0..inode_count).map(|num| INode::new(num, now)).collect::<Vec<_>>();
        INodeMap { vec: nodes }
    }

    fn find_free(&self) -> Option<usize> {
        self.vec
            .iter()
            .enumerate()
            .find(|&(_,inode)| inode.flags == INodeFlags::FREE)
            .map(|(i,_)| i)
    }

    pub fn alloc(&mut self, flags: INodeFlags) -> Option<BlockNumber> {
        self.find_free().map(|i| {
            self.vec[i].flags = flags;
            BlockNumber::new(i as u64)
        })
    }

    pub fn free(&mut self, block_number: BlockNumber) {
        self.vec[block_number.index()].flags = INodeFlags::FREE
    }
}

impl Display for INodeMap {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        fn display(inode: &INode) -> char {
            let flags = inode.flags;
            if flags.contains(INodeFlags::FREE) {
                '0'
            } else if flags.contains(INodeFlags::FILE) {
                'f'
            } else if flags.contains(INodeFlags::DIR) {
                'd'
            } else if flags.contains(INodeFlags::LINK) {
                's'
            } else if flags.contains(INodeFlags::PTR) {
                'b'
            } else if flags.contains(INodeFlags::DATA) {
                'D'
            } else {
                '0'
            }
            // match inode.flags {
            //     INodeFlags::FREE => '0',
            //     INodeFlags::FILE => 'f',
            //     INodeFlags::DIR  => 'd',
            //     INodeFlags::LINK => 's',
            //     INodeFlags::PTR  => 'b',
            //     INodeFlags::DATA => 'D'
            // }
        }
        display_chunks(self.vec.iter(), display, f)
    }
}

pub struct FileSystem {
    pub master_block: MasterBlock,
    pub block_map:    BlockMap,
    pub inode_map:    INodeMap,
        device:       BlockDevice
}

pub struct Mount {
    pub file_system: FileSystem,
    pub clean_mount: bool
}

impl FileSystem {
    pub fn new(device: BlockDevice) -> FileSystem {
        const INODE_COUNT : u16 = 8;
        let block_size = device.config.block_size;
        let block_count = device.config.block_count;
        let mut block_map = BlockMap::new(block_count);
        let master_block = MasterBlock::new(block_size, block_count, INODE_COUNT);
        for i in 0 .. 2 + master_block.block_map_blocks() + INODE_COUNT as usize {
            block_map.vec.set(i, true);
        }
        let inode_map = INodeMap::new(INODE_COUNT);
        FileSystem { master_block, block_map, inode_map, device }
    }

    pub fn write(&mut self) -> device::Result<()> {
        let master_block = &self.master_block;
        let mut bm_vec = vec![0u8; master_block.block_size as usize];
        let mut block_number = master_block.block_map;
        let mut i = 0;
        for b in self.block_map.vec.to_bytes() {
            bm_vec[i] = b;
            i += 1;
            if i == master_block.block_size as usize {
                self.device.write(block_number, &mut bm_vec)?;
                block_number.next();
                i = 0;
            }
        }
        if i != 0 {
            for j in i .. master_block.block_size as usize {
                bm_vec[j] = 0;
            }
            self.device.write(block_number, &mut bm_vec)?;
        }
        assert!(block_number < master_block.inode_map);
        block_number = master_block.inode_map;
        for node in self.inode_map.vec.iter() {
            let mut node_bytes = vec![0u8; master_block.block_size as usize];
            serialize_into(&mut node_bytes[..], &node)?;
            self.device.write(block_number, &mut node_bytes)?;
            block_number.next();
        }
        Ok(())
    }

    pub fn read(mut device: BlockDevice) -> device::Result<Mount> {
        let mut mb_vec = vec![0; device.config.block_size as usize];
        device.read(MASTER_BLOCK_NUMBER, &mut mb_vec)?;
        let mut master_block : MasterBlock = deserialize_from(&mb_vec[..])?;
        let mut bit_vec = BitVec::new();
        let mut block_number = master_block.block_map;
        for _ in 0 .. 1 + master_block.block_map_blocks() {
            let mut bm_vec = vec![0; master_block.block_size as usize];
            device.read(block_number, &mut bm_vec)?;
            block_number.next();
            bit_vec.extend(BitVec::from_bytes(&bm_vec));
        }
        bit_vec.truncate(master_block.block_count as usize);
        let block_map = BlockMap::init(bit_vec);
        assert!(block_number <= master_block.inode_map);
        let mut nodes = vec![];
        let mut block_number = master_block.inode_map;
        for _ in 0 .. master_block.inode_count {
            let mut node_bytes = vec![0u8; master_block.block_size as usize];
            device.read(block_number, &mut node_bytes)?;
            block_number.next();
            let node = deserialize_from(&node_bytes[..])?;
            nodes.push(node);
        }
        let inode_map = INodeMap { vec: nodes };
        let clean_mount = master_block.flags.contains(MasterBlockFlags::SYNCED);
        master_block.write_sync_status(&mut device, false)?;
        let file_system = FileSystem { master_block, block_map, inode_map, device };
        Ok(Mount { file_system, clean_mount })
    }

    pub fn close(mut self) -> device::Result<()> {
        self.write()?;
        self.master_block.write_sync_status(&mut self.device, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode::*;

    #[test]
    fn inode_seralize_len() {
        let now = SystemTime::now();
        let inode = INode::new(32, now);
        let v = serialize(&inode).unwrap();
        assert_eq!(126, v.len())
    }
}
