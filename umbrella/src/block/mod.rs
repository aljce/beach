use std::fmt::{self, Display, Formatter};
use std::time::{SystemTime};
use bit_vec::BitVec;
use bincode::{serialize_into, deserialize_from};

pub mod device;
use self::device::{BlockNumber, BlockDevice, MASTER_BLOCK_NUMBER};

bitflags! {
    #[derive(Serialize, Deserialize)]
    struct BlockFlags: u8 {
        const SYNCED = 0b10000000;
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MasterBlock {
    block_size:  u16,
    block_count: u64,
    inode_count: u16,
    block_map:   BlockNumber,
    inode_map:   BlockNumber,
    flags:       BlockFlags,
}

impl MasterBlock {
    pub fn new(block_size: u16, block_count: u64, inode_count: u16) -> MasterBlock {
        MasterBlock {
            block_size,
            block_count,
            inode_count,
            block_map: BlockNumber::new(2),
            inode_map: BlockNumber::new(2 + block_count / block_size as u64),
            flags:     BlockFlags::SYNCED
        }
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
}

impl Display for BlockMap {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        let mut i = 0;
        let mut s = String::new();
        for b in self.vec.iter() {
            s.push(if b { '1'} else { '0' });
            if i != 0 && i % 7 == 0 {
                s.push('|');
            }
            i += 1;
            if i == 64 {
                writeln!(f, "{}", s)?;
                s = String::new();
                i = 0;
            }
        }
        if i != 0 {
            writeln!(f, "{}", s)?
        }
        Ok(())
    }
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    struct INodeFlags: u8 {
        const FREE = 0b1000_0000;
        const FILE = 0b0100_0000;
        const DIR  = 0b0010_0000;
        const LINK = 0b0001_0000;
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
}

pub struct FileSystem {
    pub master_block: MasterBlock,
    pub block_map:    BlockMap,
    pub inode_map:    INodeMap
}

impl FileSystem {
    pub fn new<'a>(device: &BlockDevice<'a>) -> FileSystem {
        const INODE_COUNT : u16 = 8;
        let block_size = device.config.block_size;
        let block_count = device.config.block_count;
        let mut block_map = BlockMap::new(block_count);
        let block_map_blocks = (block_count as usize / block_size as usize) / 8;
        for i in 0 .. 2 + block_map_blocks + INODE_COUNT as usize {
            block_map.vec.set(i, true);
        }
        FileSystem {
            master_block: MasterBlock::new(block_size, block_count, INODE_COUNT),
            inode_map:    INodeMap::new(INODE_COUNT),
            block_map
        }
    }

    pub fn write<'a>(&self, device: &mut BlockDevice<'a>) -> device::Result<()> {
        let mb = &self.master_block;
        let mut mb_vec = vec![0; mb.block_size as usize];
        serialize_into(&mut mb_vec[..], &mb)?;
        device.write(MASTER_BLOCK_NUMBER, &mut mb_vec[..])?;

        let mut bm_vec = vec![0u8; mb.block_size as usize];
        let mut block_number = mb.block_map;
        let mut i = 0;
        for b in self.block_map.vec.to_bytes() {
            bm_vec[i] = b;
            i += 1;
            if i == mb.block_size as usize {
                device.write(block_number, &mut bm_vec)?;
                block_number.next();
                i = 0;
            }
        }
        if i != 0 {
            for j in i .. mb.block_size as usize {
                bm_vec[j] = 0;
            }
            device.write(block_number, &mut bm_vec)?;
        }
        assert!(block_number <= mb.inode_map);
        block_number = mb.inode_map;
        for node in self.inode_map.vec.iter() {
            let mut node_bytes = vec![0u8; mb.block_size as usize];
            serialize_into(&mut node_bytes[..], &node)?;
            device.write(block_number, &mut node_bytes)?;
            block_number.next();
        }
        Ok(())
    }

    pub fn read<'a>(mut device: BlockDevice<'a>) -> device::Result<FileSystem> {
        let mut mb_vec = vec![0; device.config.block_size as usize];
        device.read(MASTER_BLOCK_NUMBER, &mut mb_vec)?;
        let master_block : MasterBlock = deserialize_from(&mb_vec[..])?;
        let mut bit_vec = BitVec::new();
        let mut block_number = master_block.block_map;
        for _ in 0 .. master_block.block_count / master_block.block_size as u64 {
            let mut bm_vec = vec![0; master_block.block_size as usize];
            device.read(block_number, &mut bm_vec)?;
            block_number.next();
            bit_vec.extend(BitVec::from_bytes(&bm_vec));
        }
        let block_map = BlockMap { vec: bit_vec };
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
        Ok(FileSystem { master_block, block_map, inode_map })
    }

    pub fn write_sync_status(&mut self, device: &mut BlockDevice, status: bool) -> device::Result<()> {
        let mut master_block = self.master_block.clone();
        master_block.flags.set(BlockFlags::SYNCED, status);
        let mut mb_vec = vec![0; master_block.block_size as usize];
        serialize_into(&mut mb_vec[..], &master_block)?;
        device.write(MASTER_BLOCK_NUMBER, &mut mb_vec[..])?;
        self.master_block = master_block;
        Ok(())
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
