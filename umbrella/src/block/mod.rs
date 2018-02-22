use std::time::SystemTime;
use bit_vec::BitVec;

pub mod device;
use self::device::BlockNumber;

bitflags! {
    #[derive(Serialize, Deserialize)]
    struct BlockFlags: u8 {
        const SYNCED = 0b10000000;
    }
}

#[derive(Serialize, Deserialize)]
pub struct MasterBlock {
    block_size:  u16,
    block_count: u32,
    inode_count: u16,
    block_map:   BlockNumber,
    inode_map:   BlockNumber,
    flags:       BlockFlags,
}

// Theoretically this should be a doubly linked free list as the asymptotics on all the operations
// I want to support would be optimal. I choose a bitvec even though its asymptotics are worse. I
// did this because bitvecs have much lower constants on all the operations in question.
pub struct BlockMap {
    block_map: BitVec,

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
    block_ptrs: [BlockNumber; 16]
}

pub struct INodeMap {
    inode_map: Vec<INode>
}
