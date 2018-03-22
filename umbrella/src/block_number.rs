use std::fmt::{self, Debug, Display, Formatter};
use std::result;

// `BlockNumber` is an abstraction designed to encapsulate indexing into the disk
// It reduces the likely hood of conjuring a random block number. It does this bytes
// making creating a `BlockNumber` explicit.
// Also the field is only visible for this crate. This means that `BlockNumber` is
// immutable from the perspective of other libraries.
#[derive(Copy, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct BlockNumber {
    pub (crate) number: u64
}

impl BlockNumber {
    pub fn new(number: u64) -> BlockNumber {
        BlockNumber { number }
    }

    pub (crate) fn next(&mut self) {
        self.number += 1;
    }

    pub (crate) fn index(&self) -> usize {
        self.number as usize
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

pub const MASTER_BLOCK_NUMBER : BlockNumber = BlockNumber { number: 0 };

pub struct Sequence {
    current: BlockNumber,
    end:     u64
}

impl Sequence {
    pub fn new(start: BlockNumber, end: u64) -> Sequence {
        Sequence { current: start, end }
    }
}

impl Iterator for Sequence {
    type Item = BlockNumber;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current.number == self.end {
            None
        } else {
            let res = self.current;
            self.current.next();
            Some(res)
        }
    }
}
