use std::mem;
use std::collections::hash_map::{HashMap, Entry};

use block_number::{BlockNumber};
use device::{self, BlockDevice, Error};

pub enum CacheEntry {
    Block {
        block: Vec<u8>
    },
    Pointers {
        pointers: Vec<BlockNumber>
    }
}

fn to_u8<T>(v: Vec<T>) -> Vec<u8> {
    let data = v.as_ptr();
    let len = v.len();
    let capacity = v.capacity();
    let element_size = mem::size_of::<T>();

    // Don't allow the current vector to be dropped
    // (which would invalidate the memory)
    mem::forget(v);
    unsafe {
        // LAST-AUDIT: mckean.kylej@gmail.com 26-04-18
        Vec::from_raw_parts(
            data as *mut u8,
            len * element_size,
            capacity * element_size,
        )
    }
}

unsafe fn from_u8<T>(v: Vec<u8>) -> Vec<T> {
    // LAST-AUDIT: mckean.kylej@gmail.com 26-04-18
    let data = v.as_ptr();
    let len = v.len();
    let capacity = v.capacity();
    let element_size = mem::size_of::<T>();

    // Make sure we have a proper amount of capacity
    assert_eq!(capacity % element_size, 0);
    // Make sure we are going to read a full chunk of stuff
    assert_eq!(len % element_size, 0);

    // Don't allow the current vector to be dropped
    // (which would invalidate the memory)
    mem::forget(v);

    Vec::from_raw_parts(
        data as *mut T,
        len / element_size,
        capacity / element_size,
    )
}

impl CacheEntry {
    pub fn bytes(&self) -> Vec<u8> {
        use self::CacheEntry::*;
        match *self {
            Block { ref block } => {
                block.clone()
            }
            Pointers { ref pointers } => {
                let res = pointers.clone();
                to_u8(res)
            }
        }
    }
}



pub struct Cache {
    pub (crate) entries: HashMap<BlockNumber, CacheEntry>,
    pub (crate) device:  BlockDevice
}

impl Cache {
    pub fn new(device: BlockDevice) -> Cache {
        Cache {
            entries: HashMap::new(),
            device
        }
    }

    pub fn read(&mut self, block_num: BlockNumber) -> device::Result<Vec<u8>> {
        use self::CacheEntry::*;
        let block_size = self.device.config.block_size as usize;
        let Cache { ref mut entries, ref mut device } = *self;
        match entries.entry(block_num) {
            Entry::Occupied(o) => {
                match *o.get() {
                    Block { ref block } => {
                        Ok(block.clone())
                    }
                    Pointers { .. } => {
                        Err(Error::CacheInvalid)
                    }
                }
            }
            Entry::Vacant(v) => {
                let mut block = vec![0; block_size];
                device.read(block_num, &mut block)?;
                let cache_entry = Block { block: block.clone() };
                v.insert(cache_entry);
                Ok(block)
            }
        }
    }

    pub fn read_pointers(&mut self, block_num: BlockNumber) -> device::Result<Vec<BlockNumber>> {
        use self::CacheEntry::*;
        let block_size = self.device.config.block_size as usize;
        let Cache { ref mut entries, ref mut device } = *self;
        match entries.entry(block_num) {
            Entry::Occupied(o) => {
                match *o.get() {
                    Block { .. } => {
                        Err(Error::CacheInvalid)
                    }
                    Pointers { ref pointers } => {
                        Ok(pointers.clone())
                    }
                }
            }
            Entry::Vacant(v) => {
                let mut block = vec![0; block_size];
                device.read(block_num, &mut block)?;
                let pointers = unsafe {
                    // LAST-AUDIT: mckean.kylej@gmail.com 01-05-18
                    from_u8(block)
                };
                let cache_entry = Pointers { pointers: pointers.clone() };
                v.insert(cache_entry);
                Ok(pointers)
            }
        }
    }

    pub fn write_pointers(&mut self, block_num: BlockNumber, pointers: Vec<BlockNumber>) {
        use self::CacheEntry::*;
        self.entries.insert(block_num, Pointers { pointers });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn as_u8() {
        let vec : Vec<u64> = vec![0, 1, 2u64.pow(20) - 1, 3];
        let bytes = unsafe {
            super::from_u8(super::to_u8(vec.clone()))
        };
        assert_eq!(vec, bytes);
    }
}
