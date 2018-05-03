use std::fmt::{self, Display, Formatter};
use std::time::{SystemTime};
use bit_vec::BitVec;
use bincode::{serialize_into, deserialize_from};

use block_number::{BlockNumber, BlockOffset, MASTER_BLOCK_NUMBER, Step, Sequence};
use device::{self, BlockDevice, Error};
use cache::{Cache};

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
            block_map: BlockNumber::new(1),
            inode_map: BlockNumber::new(2 + (block_count / block_size as u64) / 8),
            flags:     MasterBlockFlags::SYNCED
        }
    }

    pub fn block_map_blocks(&self) -> u64 {
        1 + self.block_count / self.block_size as u64 / 8
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
    vec: BitVec,
}

impl BlockMap {
    pub fn new(block_count: u64) -> BlockMap {
        BlockMap {
            vec: BitVec::from_elem(block_count as usize, false)
        }
    }

    pub fn set(&mut self, block_number: BlockNumber, b: bool) {
        self.vec.set(block_number.index(), b)
    }

    fn find_free(vec: &BitVec) -> Option<usize> {
        vec.iter().enumerate().find(|&(_, b)| b == false).map(|(i, _)| i)
    }

    pub fn alloc(&mut self) -> device::Result<BlockNumber> {
        match BlockMap::find_free(&self.vec) {
            Some(i) => {
                let block_number = BlockNumber::new(i as u64);
                self.set(block_number, true);
                Ok(block_number)
            }
            None => {
                Err(Error::Size("out of blocks".to_string()))
            }
        }
    }

    pub fn free(&mut self, block_number: BlockNumber) {
        self.set(block_number, false)
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
    cdate:      SystemTime,
    mdate:      SystemTime,
    flags:      INodeFlags,
    perms:      Permissions,
    length:     u64,
    level:      u8,
    block_ptrs: [BlockNumber; 8]
}

impl INode {
    pub fn new(now: SystemTime) -> INode {
        INode {
            cdate: now,
            mdate: now,
            flags: INodeFlags::FREE,
            perms: Permissions::UNUSED,
            length: 0,
            level: 0,
            block_ptrs: [BlockNumber::new(0); 8]
        }
    }
}

pub struct INodeMap {
    vec: Vec<INode>
}

impl INodeMap {
    pub fn new(inode_count: u16) -> INodeMap {
        let now = SystemTime::now();
        let nodes = (0..inode_count).map(|_| INode::new(now)).collect::<Vec<_>>();
        INodeMap { vec: nodes }
    }

    fn find_free(&self) -> Option<usize> {
        self.vec
            .iter()
            .enumerate()
            .find(|&(_,inode)| inode.flags == INodeFlags::FREE)
            .map(|(i,_)| i)
    }

    pub fn alloc(&mut self, flags: INodeFlags) -> Option<usize> {
        self.find_free().map(move |i| {
            let inode = &mut self.vec[i];
            inode.flags = flags;
            inode.mdate = SystemTime::now();
            i
        })
    }

    pub fn get(&self, index: usize) -> &INode {
        &self.vec[index]
    }

    pub fn get_mut(&mut self, index: usize) -> &mut INode {
        &mut self.vec[index]
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
        cache:        Cache
}

pub struct Mount {
    pub file_system: FileSystem,
    pub clean_mount: bool
}

impl FileSystem {
    pub fn new(device: BlockDevice) -> FileSystem {
        const INODE_COUNT : u16 = 10;
        let block_size = device.config.block_size;
        let block_count = device.config.block_count;
        let mut block_map = BlockMap::new(block_count);
        let master_block = MasterBlock::new(block_size, block_count, INODE_COUNT);
        let claimed_blocks = master_block.block_map_blocks() + INODE_COUNT as u64;
        for i in Sequence::new(MASTER_BLOCK_NUMBER, claimed_blocks) {
            block_map.set(i, true);
        }
        let inode_map = INodeMap::new(INODE_COUNT);
        let cache = Cache::new(device);
        FileSystem { master_block, block_map, inode_map, cache }
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
                self.cache.device.write(block_number, &mut bm_vec)?;
                block_number.inc();
                i = 0;
            }
        }
        if i != 0 {
            for j in i .. master_block.block_size as usize {
                bm_vec[j] = 0;
            }
            self.cache.device.write(block_number, &mut bm_vec)?;
        }
        assert!(block_number < master_block.inode_map);
        block_number = master_block.inode_map;
        for node in self.inode_map.vec.iter() {
            let mut node_bytes = vec![0u8; master_block.block_size as usize];
            serialize_into(&mut node_bytes[..], &node)?;
            self.cache.device.write(block_number, &mut node_bytes)?;
            block_number.inc();
        }
        for (block_number, cache_entry) in &self.cache.entries {
            self.cache.device.write(*block_number, &mut cache_entry.bytes())?
        }
        Ok(())
    }

    pub fn read(mut device: BlockDevice) -> device::Result<Mount> {
        let mut mb_vec = vec![0; device.config.block_size as usize];
        device.read(MASTER_BLOCK_NUMBER, &mut mb_vec)?;
        let mut master_block : MasterBlock = deserialize_from(&mb_vec[..])?;
        let mut bit_vec = BitVec::new();
        let mut block_number = master_block.block_map;
        for _ in 1 .. master_block.block_map_blocks() + 1 {
            let mut bm_vec = vec![0; master_block.block_size as usize];
            device.read(block_number, &mut bm_vec)?;
            block_number.inc();
            bit_vec.extend(BitVec::from_bytes(&bm_vec));
        }
        bit_vec.truncate(master_block.block_count as usize);
        let block_map = BlockMap { vec: bit_vec };
        assert!(block_number <= master_block.inode_map);
        let mut nodes = vec![];
        let mut block_number = master_block.inode_map;
        for _ in 0 .. master_block.inode_count {
            let mut node_bytes = vec![0u8; master_block.block_size as usize];
            device.read(block_number, &mut node_bytes)?;
            block_number.inc();
            let node = deserialize_from(&node_bytes[..])?;
            nodes.push(node);
        }
        let inode_map = INodeMap { vec: nodes };
        let clean_mount = master_block.flags.contains(MasterBlockFlags::SYNCED);
        master_block.write_sync_status(&mut device, false)?;
        let cache = Cache::new(device);
        let file_system = FileSystem { master_block, block_map, inode_map, cache };
        Ok(Mount { file_system, clean_mount })
    }

    pub fn close(mut self) -> device::Result<()> {
        self.write()?;
        self.master_block.write_sync_status(&mut self.cache.device, true)
    }

    pub fn lookup_block_num_from_offset<'a>(&mut self, inode_num: usize, offset: BlockOffset) ->
        device::Result<Option<BlockNumber>>
    {
        fn rec(cache: &mut Cache, offset: BlockOffset, block_ptrs: &[BlockNumber], level: u8) ->
            device::Result<Option<BlockNumber>>
        {
            if level == 0 {
                if offset < block_ptrs.len() {
                    let block_num = block_ptrs[offset.index()];
                    if block_num == MASTER_BLOCK_NUMBER {
                        Ok(None)
                    } else {
                        Ok(Some(block_num))
                    }
                } else {
                    Err(Error::Overflow)
                }
            } else {
                let bnpl = cache.device.block_numbers_per_level(level);
                let next_offset = offset % bnpl;
                let next_block  = offset / bnpl;
                let next_block_index = block_ptrs[next_block.index()];
                if next_block_index == MASTER_BLOCK_NUMBER {
                    Ok(None)
                } else {
                    let next_block_ptrs = cache.read_pointers(next_block_index)?;
                    rec(cache, next_offset, &next_block_ptrs, level - 1)
                }
            }
        }
        let inode = self.inode_map.get(inode_num);
        rec(&mut self.cache, offset, &inode.block_ptrs, inode.level)
    }

    pub fn alloc_block_num_from_offset(&mut self, inode_num: usize, offset: BlockOffset) ->
        device::Result<Option<BlockNumber>>
    {
        fn rec(block_map: &mut BlockMap,
               cache: &mut Cache,
               offset: BlockOffset,
               block_ptrs: &mut [BlockNumber],
               level: u8) ->
            device::Result<Option<BlockNumber>>
        {
            if level == 0 {
                if offset < block_ptrs.len() {
                    let block_num = block_ptrs[offset.index()];
                    if block_num == MASTER_BLOCK_NUMBER {
                        let new_block_num = block_map.alloc()?;
                        block_ptrs[offset.index()] = new_block_num;
                        Ok(Some(new_block_num))
                    } else {
                        Ok(Some(block_num))
                    }
                } else {
                    Err(Error::Overflow)
                }
            } else {
                let bnpl = cache.device.block_numbers_per_level(level);
                let next_offset = offset % bnpl;
                let next_block  = offset / bnpl;
                let next_block_index = block_ptrs[next_block.index()];
                if next_block_index == MASTER_BLOCK_NUMBER {
                    Ok(None)
                } else {
                    let mut next_block_ptrs = cache.read_pointers(next_block_index)?;
                    rec(block_map, cache, next_offset, &mut next_block_ptrs, level - 1)
                }
            }
        }
        let inode = self.inode_map.get_mut(inode_num);
        let bnpl = self.cache.device.block_numbers_per_level(inode.level);
        if offset >= inode.block_ptrs.len() * bnpl {
            let new_block_num = self.block_map.alloc()?;
            let mut new_block = vec![MASTER_BLOCK_NUMBER; self.cache.device.block_numbers_per_block()];
            let mut i = 0;
            for block_num in inode.block_ptrs.iter() {
                new_block[i] = *block_num;
                i += 1;
            }
            self.cache.write_pointers(new_block_num, new_block);
            let mut new_block_ptrs = [MASTER_BLOCK_NUMBER; 8];
            new_block_ptrs[0] = new_block_num;
            inode.block_ptrs = new_block_ptrs;
            inode.level += 1;
        }
        let r = rec(&mut self.block_map, &mut self.cache, offset, &mut inode.block_ptrs, inode.level)?;
        inode.length += 1;
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode::*;

    #[test]
    fn inode_seralize_len() {
        let now = SystemTime::now();
        let inode = INode::new(now);
        let v = serialize(&inode).unwrap();
        assert!(v.len() < 128)
    }

    #[test]
    fn inode_alloc_read_simple() {
        let device = BlockDevice::create("foo", 128, Some(128)).unwrap();
        let mut fs = FileSystem::new(device);
        let zero   = BlockOffset::new(0);
        let inode_num = fs.inode_map.alloc(INodeFlags::FILE).unwrap();
        let alloced_block_num = fs.alloc_block_num_from_offset(inode_num, zero).unwrap();
        let stored_block_num = fs.lookup_block_num_from_offset(inode_num, zero).unwrap();
        assert_eq!(alloced_block_num, stored_block_num)
    }

    #[test]
    fn inode_alloc_read_many() {
        let device = BlockDevice::create("foo", 128, Some(128)).unwrap();
        let mut fs = FileSystem::new(device);
        let inode_num = fs.inode_map.alloc(INodeFlags::FILE).unwrap();
        let seq = Sequence::new(BlockOffset::zero(), 20);
        let alloced_block_nums =
            seq.map(|i| fs.alloc_block_num_from_offset(inode_num, i).unwrap()).collect::<Vec<_>>();
        let stored_block_nums =
            seq.map(|i| fs.lookup_block_num_from_offset(inode_num, i).unwrap()).collect::<Vec<_>>();
        assert_eq!(alloced_block_nums, stored_block_nums);
    }
}
