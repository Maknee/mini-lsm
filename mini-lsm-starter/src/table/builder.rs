#![allow(unused_variables)] // TODO(you): remove this lint after implementing this mod
#![allow(dead_code)] // TODO(you): remove this lint after implementing this mod

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use bytes::{BufMut, Bytes};

use super::{BlockMeta, FileObject, SsTable};
use crate::{block::BlockBuilder, key::KeyBytes, key::KeySlice, lsm_storage::BlockCache};

/// Builds an SSTable from key-value pairs.
pub struct SsTableBuilder {
    builder: BlockBuilder,
    first_key: Vec<u8>,
    last_key: Vec<u8>,
    data: Vec<u8>,
    pub(crate) meta: Vec<BlockMeta>,
    block_size: usize,
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
        }
    }

    /// Adds a key-value pair to SSTable.
    ///
    /// Note: You should split a new block when the current block is full.(`std::mem::replace` may
    /// be helpful here)
    pub fn add(&mut self, key: KeySlice, value: &[u8]) {
        if self.builder.is_empty() {
            self.first_key = key.raw_ref().to_vec();
        }
        self.last_key = key.raw_ref().to_vec();
        if self.builder.add(key, value) {
            // add meta
            let block_meta = BlockMeta {
                offset: self.data.len(),
                first_key: KeyBytes::from_bytes(Bytes::from(self.first_key.clone())),
                last_key: KeyBytes::from_bytes(Bytes::from(self.last_key.clone())),
            };
            self.meta.push(block_meta);

            self.first_key.clear();
            self.last_key.clear();

            // add block
            let builder = std::mem::replace(&mut self.builder, BlockBuilder::new(self.block_size));
            let block = builder.build();
            self.data.put(block.encode());
        }
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
        self,
        id: usize,
        block_cache: Option<Arc<BlockCache>>,
        path: impl AsRef<Path>,
    ) -> Result<SsTable> {
        let mut data = vec![];
        data.put(self.data.as_ref());
        BlockMeta::encode_block_meta(&self.meta, &mut data);
        data.put_u32(self.data.len() as u32);
        let file_object = FileObject::create(path.as_ref(), data)?;
        SsTable::open(id, block_cache, file_object)
    }

    #[cfg(test)]
    pub(crate) fn build_for_test(self, path: impl AsRef<Path>) -> Result<SsTable> {
        self.build(0, None, path)
    }
}
