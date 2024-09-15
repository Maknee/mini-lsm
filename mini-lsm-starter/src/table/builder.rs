#![allow(unused_variables)] // TODO(you): remove this lint after implementing this mod
#![allow(dead_code)] // TODO(you): remove this lint after implementing this mod

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use bytes::{BufMut, Bytes};

use super::{BlockMeta, FileObject, SsTable};
use crate::{
    block::BlockBuilder, key::KeyBytes, key::KeySlice, lsm_storage::BlockCache, table::Bloom,
};

/// Builds an SSTable from key-value pairs.
pub struct SsTableBuilder {
    builder: BlockBuilder,
    first_key: Vec<u8>,
    last_key: Vec<u8>,
    data: Vec<u8>,
    pub(crate) meta: Vec<BlockMeta>,
    block_size: usize,
    key_hashes: Vec<u32>,
}

impl SsTableBuilder {
    /// Create a builder based on target block size.
    pub fn new(block_size: usize) -> Self {
        Self {
            builder: BlockBuilder::new(block_size),
            first_key: Vec::new(),
            last_key: Vec::new(),
            data: Vec::new(),
            meta: Vec::new(),
            block_size,
            key_hashes: Vec::new(),
        }
    }

    /// Adds a key-value pair to SSTable.
    ///
    /// Note: You should split a new block when the current block is full.(`std::mem::replace` may
    /// be helpful here)
    pub fn add(&mut self, key: KeySlice, value: &[u8]) {
        if self.first_key.is_empty() {
            self.first_key = key.raw_ref().to_vec();
        }

        self.key_hashes.push(farmhash::fingerprint32(key.raw_ref()));

        if self.builder.add(key, value) {
            self.last_key = key.raw_ref().to_vec();
            return;
        }

        self.finish_block();

        assert!(self.builder.add(key, value));
        self.first_key = key.raw_ref().to_vec();
        self.last_key = key.raw_ref().to_vec();
    }

    fn finish_block(&mut self) {
        let builder = std::mem::replace(&mut self.builder, BlockBuilder::new(self.block_size));
        let encoded_block = builder.build().encode();
        self.meta.push(BlockMeta {
            offset: self.data.len(),
            first_key: KeyBytes::from_bytes(Bytes::from(std::mem::take(&mut self.first_key))),
            last_key: KeyBytes::from_bytes(Bytes::from(std::mem::take(&mut self.last_key))),
        });
        self.data.extend(encoded_block);
    }

    /// Get the estimated size of the SSTable.
    ///
    /// Since the data blocks contain much more data than meta blocks, just return the size of data
    /// blocks here.
    pub fn estimated_size(&self) -> usize {
        self.data.len()
    }

    /// Builds the SSTable and writes it to the given path. Use the `FileObject` structure to manipulate the disk objects.
    pub fn build(
        mut self,
        id: usize,
        block_cache: Option<Arc<BlockCache>>,
        path: impl AsRef<Path>,
    ) -> Result<SsTable> {
        self.finish_block();

        let mut data = self.data;

        // data blocks
        let data_block_len = data.len();

        // index blocks
        let meta_block_off = data.len();
        BlockMeta::encode_block_meta(&self.meta, &mut data);
        let meta_block_len = data.len() - meta_block_off;

        // bloom filter
        let bits_per_key = Bloom::bloom_bits_per_key(self.key_hashes.len(), 0.01);
        let bloom = Bloom::build_from_key_hashes(&self.key_hashes, bits_per_key);
        let bloom_block_off = data.len();
        bloom.encode(&mut data);
        let bloom_block_len = data.len() - bloom_block_off;

        // Meta datas

        // meta block len
        data.put_u32(meta_block_len as u32);
        // meta block file offset
        data.put_u32(meta_block_off as u32);

        // bloom block len
        data.put_u32(bloom_block_len as u32);
        // bloom block file offset
        data.put_u32(bloom_block_off as u32);

        let file_object = FileObject::create(path.as_ref(), data)?;
        SsTable::open(id, block_cache, file_object)
    }

    #[cfg(test)]
    pub(crate) fn build_for_test(self, path: impl AsRef<Path>) -> Result<SsTable> {
        self.build(0, None, path)
    }
}
