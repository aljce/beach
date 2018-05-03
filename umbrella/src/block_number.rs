use std::fmt::{self, Debug, Display, Formatter};
use std::cmp::Ordering;
use std::result;
use std::ops::{Rem, Div};

// `BlockNumber` is an abstraction designed to encapsulate indexing into the disk
// It reduces the likely hood of conjuring a random block number. It does this bytes
// making creating a `BlockNumber` explicit.
// Also the field is only visible for this crate. This means that `BlockNumber` is
// immutable from the perspective of other libraries.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BlockNumber {
    pub (crate) number: u64
}

pub const MASTER_BLOCK_NUMBER : BlockNumber = BlockNumber { number: 0 };

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

#[derive(Copy, Clone)]
pub struct BlockOffset {
    offset: u64
}

impl BlockOffset {
    pub fn new(offset: u64) -> BlockOffset {
        BlockOffset { offset }
    }

    pub fn index(&self) -> usize {
        self.offset as usize
    }
}

impl PartialEq<usize> for BlockOffset {
    fn eq(&self, other: &usize) -> bool {
        self.offset == *other as u64
    }
}

impl PartialOrd<usize> for BlockOffset {
    fn partial_cmp(&self, other: &usize) -> Option<Ordering> {
        self.offset.partial_cmp(&(*other as u64))
    }
}

impl Div<usize> for BlockOffset {
    type Output = Self;
    fn div(mut self, divisor: usize) -> Self::Output {
        self.offset = self.offset / divisor as u64;
        self
    }
}

impl Rem<usize> for BlockOffset {
    type Output = Self;
    fn rem(mut self, modulus: usize) -> Self::Output {
        self.offset = self.offset % modulus as u64;
        self
    }
}
