#![allow(unused_variables)] // TODO(you): remove this lint after implementing this mod
#![allow(dead_code)] // TODO(you): remove this lint after implementing this mod

use crate::key::{KeySlice, KeyVec};
use bytes::BufMut;

use super::{Block, LEN_SIZE};

/// Builds a block.
pub struct BlockBuilder {
    /// Offsets of each key-value entries.
    offsets: Vec<u16>,
    /// All serialized key-value pairs in the block.
    data: Vec<u8>,
    /// The expected block size.
    block_size: usize,
    /// The first key in the block
    first_key: KeyVec,
}

impl BlockBuilder {
    /// Creates a new block builder.
    pub fn new(block_size: usize) -> Self {
        Self {
            offsets: vec![],
            data: Vec::with_capacity(block_size),
            block_size,
            first_key: KeyVec::new(),
        }
    }

    fn estimated_size(&self) -> usize {
        LEN_SIZE + (self.offsets.len() * LEN_SIZE) + self.data.len()
    }

    /// Adds a key-value pair to the block. Returns false when the block is full.
    #[must_use]
    pub fn add(&mut self, key: KeySlice, value: &[u8]) -> bool {
        let offset = self.data.len();
        let key_len = key.raw_ref().len();
        let value_len = value.len();
        let total_size = self.estimated_size() + key_len + value_len + LEN_SIZE * 3;
        if total_size > self.block_size && !self.is_empty() {
            return false;
        }
        let mut key_overlap_len = 0;
        for (x, y) in self.first_key.raw_ref().iter().zip(key.raw_ref()) {
            if x == y {
                key_overlap_len += 1;
            } else {
                break;
            }
        }

        let rest_key = &key.raw_ref()[key_overlap_len..];
        let rest_key_len = rest_key.len();

        // key length
        self.data.put_u16(key_overlap_len as u16);
        self.data.put_u16(rest_key_len as u16);

        // value length
        self.data.put_u16(value_len as u16);

        // key bytes
        self.data.put(rest_key);
        self.data.put(value);

        self.offsets.push(offset.try_into().unwrap());

        if self.first_key.is_empty() {
            self.first_key = key.to_key_vec();
        }

        true
    }

    /// Check if there is no key-value pair in the block.
    pub fn is_empty(&self) -> bool {
        self.offsets.is_empty()
    }

    /// Finalize the block.
    pub fn build(self) -> Block {
        Block {
            data: self.data.clone(),
            offsets: self.offsets.clone(),
        }
    }
}
