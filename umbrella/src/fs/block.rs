use std::time::SystemTime;
use bit_vec::BitVec;

bitflags! {
    struct BlockFlags: u8 {
        const SYNCED = 0b10000000;
    }
}

pub struct MasterBlock {
    block_size:  u16,
    block_count: u32,
    inode_count: u16,
    block_map:   u32,
    inode_map:   u32,
    flags:       BlockFlags,
}

pub struct BlockMap {
    block_map: BitVec
}

bitflags! {
    struct INodeFlags: u8 {
        const FREE = 0b1000_0000;
        const FILE = 0b0100_0000;
        const DIR  = 0b0010_0000;
        const LINK = 0b0001_0000;

    }
}
// I use rusts SystemTime to represent and serialize time. This type cannot be fit into
// 32 bits but that restriction is silly and wrong. See the 2038 unix-time apocalypse for details.
pub struct INode {
    inode_num:  u16,
    cdate:      SystemTime,
    mdate:      SystemTime,
    flags:      INodeFlags,
    perms:      u16,
    level:      u8,
    block_ptrs: [u32; 16]
}
