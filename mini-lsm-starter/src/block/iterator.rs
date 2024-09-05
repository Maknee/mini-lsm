#![allow(unused_variables)] // TODO(you): remove this lint after implementing this mod
#![allow(dead_code)] // TODO(you): remove this lint after implementing this mod

use std::sync::Arc;

use crate::key::{KeySlice, KeyVec};
use bytes::Buf;

use super::{Block, LEN_SIZE};

/// Iterates on a block.
pub struct BlockIterator {
    /// The internal `Block`, wrapped by an `Arc`
    block: Arc<Block>,
    /// The current key, empty represents the iterator is invalid
    key: KeyVec,
    /// the current value range in the block.data, corresponds to the current key
    value_range: (usize, usize),
    /// Current index of the key-value pair, should be in range of [0, num_of_elements)
    idx: usize,
    /// The first key in the block
    first_key: KeyVec,
}

impl BlockIterator {
    fn new(block: Arc<Block>) -> Self {
        Self {
            block,
            key: KeyVec::new(),
            value_range: (0, 0),
            idx: 0,
            first_key: KeyVec::new(),
        }
    }

    /// Creates a block iterator and seek to the first entry.
    pub fn create_and_seek_to_first(block: Arc<Block>) -> Self {
        let mut iter = Self::new(block);
        iter.seek_to_first();
        iter
    }

    /// Creates a block iterator and seek to the first key that >= `key`.
    pub fn create_and_seek_to_key(block: Arc<Block>, key: KeySlice) -> Self {
        let mut iter = Self::create_and_seek_to_first(block);
        iter.seek_to_key(key);
        iter
    }

    /// Returns the key of the current entry.
    pub fn key(&self) -> KeySlice {
        self.key.as_key_slice()
    }

    /// Returns the value of the current entry.
    pub fn value(&self) -> &[u8] {
        let data = &self.block.data;
        &data[self.value_range.0..self.value_range.1]
    }

    /// Returns true if the iterator is valid.
    /// Note: You may want to make use of `key`
    pub fn is_valid(&self) -> bool {
        !self.key.is_empty()
    }

    /// Seeks to the first key in the block.
    pub fn seek_to_first(&mut self) {
        let data = &self.block.data;
        let offsets = &self.block.offsets;
        if offsets.is_empty() {
            return;
        }

        let key_len = (&data[..]).get_u16() as usize;
        let value_len = (&data[LEN_SIZE..]).get_u16() as usize;

        let start_of_kv = LEN_SIZE * 2;
        let key = KeyVec::from_vec(data[start_of_kv..(start_of_kv + key_len)].to_vec());
        let value_range = (start_of_kv + key_len, start_of_kv + key_len + value_len);

        self.key = key.clone();
        self.value_range = value_range;
        self.idx = 0;
        self.first_key = key.clone();
    }

    /// Move to the next key in the block.
    pub fn next(&mut self) {
        let data = &self.block.data;
        let offsets = &self.block.offsets;

        let next_idx = self.idx + 1;
        if next_idx >= offsets.len() {
            self.key.clear();
            return;
            // panic!("Next has no offsets");
        }
        self.idx = next_idx;

        let offset = offsets[self.idx] as usize;

        let key_len = (&data[offset..]).get_u16() as usize;
        let value_len = (&data[offset + LEN_SIZE..]).get_u16() as usize;

        let start_of_kv = offset + (LEN_SIZE * 2);
        let start_of_value_offset = start_of_kv + key_len;
        self.key = KeyVec::from_vec(data[start_of_kv..start_of_value_offset].to_vec());
        self.value_range = (start_of_value_offset, start_of_value_offset + value_len);
    }

    /// Seek to the first key that >= `key`.
    /// Note: You should assume the key-value pairs in the block are sorted when being added by
    /// callers.
    pub fn seek_to_key(&mut self, key: KeySlice) {
        loop {
            if self.key() >= key {
                break;
            }
            self.next();
            if !self.is_valid() {
                self.seek_to_first();
                break;
            }
        }
    }
}
