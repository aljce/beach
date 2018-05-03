use std::fmt::{self, Debug, Display, Formatter};
use std::cmp::Ordering;
use std::result;
use std::ops::{Rem, Div};

pub trait Step {
    fn inc(&mut self);
}

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

    pub (crate) fn index(&self) -> usize {
        self.number as usize
    }

    pub fn seq(self, end: u64) -> Sequence<BlockNumber> {
        Sequence {
            current: self,
            end: BlockNumber::new(end)
        }
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

impl From<u64> for BlockNumber {
    fn from(number: u64) -> BlockNumber {
        BlockNumber { number }
    }
}

impl Step for BlockNumber {
    fn inc(&mut self) {
        self.number += 1;
    }
}

#[derive(Copy, Clone)]
pub struct Sequence<S> {
    current: S,
    end:     S
}

impl<S: From<u64>> Sequence<S> {
    pub fn new(start: S, end: u64) -> Sequence<S> {
        Sequence { current: start, end: S::from(end) }
    }
}

impl<S: Copy + Eq + Step> Iterator for Sequence<S> {
    type Item = S;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let res = self.current;
            self.current.inc();
            Some(res)
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BlockOffset {
    offset: u64
}

impl BlockOffset {
    pub fn new(offset: u64) -> BlockOffset {
        BlockOffset { offset }
    }

    pub fn zero() -> BlockOffset {
        BlockOffset::new(0)
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

impl From<u64> for BlockOffset {
    fn from(offset: u64) -> BlockOffset {
        BlockOffset::new(offset)
    }
}

impl Step for BlockOffset {
    fn inc(&mut self) {
        self.offset += 1;
    }
}
