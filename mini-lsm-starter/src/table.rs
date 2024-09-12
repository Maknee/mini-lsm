#![allow(unused_variables)] // TODO(you): remove this lint after implementing this mod
#![allow(dead_code)] // TODO(you): remove this lint after implementing this mod

pub(crate) mod bloom;
mod builder;
mod iterator;

use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
pub use builder::SsTableBuilder;
use bytes::{Buf, BufMut};
pub use iterator::SsTableIterator;

use crate::block::Block;
use crate::key::{KeyBytes, KeySlice};
use crate::lsm_storage::BlockCache;

use self::bloom::Bloom;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockMeta {
    /// Offset of this data block.
    pub offset: usize,
    /// The first key of the data block.
    pub first_key: KeyBytes,
    /// The last key of the data block.
    pub last_key: KeyBytes,
}

impl BlockMeta {
    /// Encode block meta to a buffer.
    /// You may add extra fields to the buffer,
    /// in order to help keep track of `first_key` when decoding from the same buffer in the future.
    pub fn encode_block_meta(
        block_meta: &[BlockMeta],
        #[allow(clippy::ptr_arg)] // remove this allow after you finish
        buf: &mut Vec<u8>,
    ) {
        buf.put_u16(block_meta.len().try_into().unwrap());
        for m in block_meta {
            buf.put_u64(m.offset as u64);
            buf.put_u16(m.first_key.raw_ref().len().try_into().unwrap());
            buf.put_u16(m.last_key.raw_ref().len().try_into().unwrap());
            buf.put(m.first_key.raw_ref());
            buf.put(m.last_key.raw_ref());
        }
    }

    /// Decode block meta from a buffer.
    pub fn decode_block_meta(buf: impl Buf) -> Vec<BlockMeta> {
        let mut buf = buf.chunk();
        let len = buf.get_u16();
        let mut block_meta = Vec::with_capacity(len.into());
        for _ in 0..len {
            let offset = buf.get_u64();
            let first_key_len = buf.get_u16();
            let last_key_len = buf.get_u16();
            let first_key = KeyBytes::from_bytes(buf.copy_to_bytes(first_key_len.into()));
            let last_key = KeyBytes::from_bytes(buf.copy_to_bytes(last_key_len.into()));

            block_meta.push(BlockMeta {
                offset: offset as usize,
                first_key,
                last_key,
            });
        }

        block_meta
    }
}

/// A file object.
pub struct FileObject(Option<File>, u64);

impl FileObject {
    pub fn read(&self, offset: u64, len: u64) -> Result<Vec<u8>> {
        use std::os::unix::fs::FileExt;
        let mut data = vec![0; len as usize];
        self.0
            .as_ref()
            .unwrap()
            .read_exact_at(&mut data[..], offset)?;
        Ok(data)
    }

    pub fn size(&self) -> u64 {
        self.1
    }

    /// Create a new file object (day 2) and write the file to the disk (day 4).
    pub fn create(path: &Path, data: Vec<u8>) -> Result<Self> {
        std::fs::write(path, &data)?;
        File::open(path)?.sync_all()?;
        Ok(FileObject(
            Some(File::options().read(true).write(true).open(path)?),
            data.len() as u64,
        ))
    }

    pub fn open(path: &Path) -> Result<Self> {
        let file = File::options().read(true).write(false).open(path)?;
        let size = file.metadata()?.len();
        Ok(FileObject(Some(file), size))
    }
}

/// An SSTable.
pub struct SsTable {
    /// The actual storage unit of SsTable, the format is as above.
    pub(crate) file: FileObject,
    /// The meta blocks that hold info for data blocks.
    pub(crate) block_meta: Vec<BlockMeta>,
    /// The offset that indicates the start point of meta blocks in `file`.
    pub(crate) block_meta_offset: usize,
    id: usize,
    block_cache: Option<Arc<BlockCache>>,
    first_key: KeyBytes,
    last_key: KeyBytes,
    pub(crate) bloom: Option<Bloom>,
    /// The maximum timestamp stored in this SST, implemented in week 3.
    max_ts: u64,
}

impl SsTable {
    #[cfg(test)]
    pub(crate) fn open_for_test(file: FileObject) -> Result<Self> {
        Self::open(0, None, file)
    }

    /// Open SSTable from a file.
    pub fn open(id: usize, block_cache: Option<Arc<BlockCache>>, file: FileObject) -> Result<Self> {
        let size = file.size();
        let block_meta_offset_raw = file.read(
            size - std::mem::size_of::<u32>() as u64,
            std::mem::size_of::<u32>() as u64,
        )?;
        let block_meta_offset = (&block_meta_offset_raw[..]).get_u32() as u64;
        let data = file.read(block_meta_offset, size - block_meta_offset)?;

        // Decode
        let block_meta = BlockMeta::decode_block_meta(&data[..]);

        let first_key = if let Some(x) = block_meta.first() {
            x.first_key.clone()
        } else {
            return Err(anyhow::anyhow!("No first key"));
        };
        let last_key = if let Some(x) = block_meta.last() {
            x.last_key.clone()
        } else {
            return Err(anyhow::anyhow!("No last key"));
        };

        Ok(Self {
            file,
            block_meta,
            block_meta_offset: block_meta_offset as usize,
            id,
            block_cache,
            first_key,
            last_key,
            bloom: None,
            max_ts: 0,
        })
    }

    /// Create a mock SST with only first key + last key metadata
    pub fn create_meta_only(
        id: usize,
        file_size: u64,
        first_key: KeyBytes,
        last_key: KeyBytes,
    ) -> Self {
        Self {
            file: FileObject(None, file_size),
            block_meta: vec![],
            block_meta_offset: 0,
            id,
            block_cache: None,
            first_key,
            last_key,
            bloom: None,
            max_ts: 0,
        }
    }

    /// Read a block from the disk.
    pub fn read_block(&self, block_idx: usize) -> Result<Arc<Block>> {
        let mut f = self.file.0.as_ref().unwrap();
        let metadata = &self.block_meta[block_idx];
        let offset = metadata.offset;
        let len = match block_idx {
            idx if idx == self.block_meta.len() - 1 => self.block_meta_offset - offset,
            idx if idx < self.block_meta.len() - 1 => self.block_meta[idx + 1].offset - offset,
            _ => return Err(anyhow::anyhow!("out of bounds")),
        };
        let mut data = vec![0; len];
        f.seek(SeekFrom::Start(offset.try_into()?))?;
        f.read_exact(&mut data)?;
        let block = Block::decode(&data);
        Ok(Arc::new(block))
    }

    /// Read a block from disk, with block cache. (Day 4)
    pub fn read_block_cached(&self, block_idx: usize) -> Result<Arc<Block>> {
        if let Some(block_cache) = &self.block_cache {
            let block =
                block_cache.try_get_with((self.id, block_idx), || self.read_block(block_idx));
            block.map_err(|x| anyhow::anyhow!(x))
        } else {
            self.read_block(block_idx)
        }
    }

    /// Find the block that may contain `key`.
    /// Note: You may want to make use of the `first_key` stored in `BlockMeta`.
    /// You may also assume the key-value pairs stored in each consecutive block are sorted.
    pub fn find_block_idx(&self, key: KeySlice) -> usize {
        for (i, metadata) in self.block_meta.iter().enumerate() {
            if metadata.first_key.as_key_slice() >= key {
                return i;
            }
        }
        // panic!("Find block index");
        0
    }

    /// Get number of data blocks.
    pub fn num_of_blocks(&self) -> usize {
        self.block_meta.len()
    }

    pub fn first_key(&self) -> &KeyBytes {
        &self.first_key
    }

    pub fn last_key(&self) -> &KeyBytes {
        &self.last_key
    }

    pub fn table_size(&self) -> u64 {
        self.file.1
    }

    pub fn sst_id(&self) -> usize {
        self.id
    }

    pub fn max_ts(&self) -> u64 {
        self.max_ts
    }
}
