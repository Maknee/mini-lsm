#![allow(unused_variables)] // TODO(you): remove this lint after implementing this mod
#![allow(dead_code)] // TODO(you): remove this lint after implementing this mod

use anyhow::Result;
use bytes::Bytes;
use std::ops::Bound;

use crate::{
    iterators::{
        merge_iterator::MergeIterator, two_merge_iterator::TwoMergeIterator, StorageIterator,
    },
    mem_table::MemTableIterator,
    table::SsTableIterator,
};

/// Represents the internal type for an LSM iterator. This type will be changed across the tutorial for multiple times.
// type LsmIteratorInner = MergeIterator<MemTableIterator>;
type LsmIteratorInner =
    TwoMergeIterator<MergeIterator<MemTableIterator>, MergeIterator<SsTableIterator>>;

pub struct LsmIterator {
    inner: LsmIteratorInner,
    end_bound: Bound<Bytes>,
    valid: bool,
    prev_key: Vec<u8>,
}

impl LsmIterator {
    pub(crate) fn new(iter: LsmIteratorInner, end_bound: Bound<Bytes>) -> Result<Self> {
        let mut iter = Self {
            valid: iter.is_valid(),
            inner: iter,
            end_bound,
            prev_key: vec![],
        };
        iter.ensure_valid()?;
        Ok(iter)
    }

    fn go_to_next(&mut self) -> Result<()> {
        self.inner.next()?;
        if !self.inner.is_valid() {
            self.valid = false;
            return Ok(());
        }
        match self.end_bound.as_ref() {
            Bound::Unbounded => {}
            Bound::Included(key) => self.valid = self.inner.key().raw_ref() <= key.as_ref(),
            Bound::Excluded(key) => self.valid = self.inner.key().raw_ref() < key.as_ref(),
        }
        Ok(())
    }

    fn ensure_valid(&mut self) -> Result<()> {
        // while self.is_valid() && self.inner.value().is_empty() {
        //     self.go_to_next()?;
        // }

        loop {
            while self.is_valid() && self.inner.key().raw_ref() == self.prev_key {
                self.go_to_next()?;
            }
            if !self.inner.is_valid() {
                break;
            }
            if *self.prev_key > *self.inner.key().raw_ref() {
                self.valid = false;
                return Ok(());
            }
            self.prev_key.clear();
            self.prev_key.extend(self.inner.key().raw_ref());
            // while self.inner.is_valid() && self.inner.key().raw_ref() == self.prev_key {
            //     self.go_to_next()?;
            // }
            if !self.inner.is_valid() {
                break;
            }
            if !self.valid {
                break;
            }
            if !self.value().is_empty() {
                break;
            }
            if self.inner.key().raw_ref() != self.prev_key {
                continue;
            }
            if !self.inner.value().is_empty() {
                break;
            }
        }
        Ok(())
    }
}

impl StorageIterator for LsmIterator {
    type KeyType<'a> = &'a [u8];

    fn is_valid(&self) -> bool {
        self.valid
    }

    fn key(&self) -> &[u8] {
        self.inner.key().raw_ref()
    }

    fn value(&self) -> &[u8] {
        self.inner.value()
    }

    fn next(&mut self) -> Result<()> {
        self.go_to_next()?;
        self.ensure_valid()?;
        Ok(())
    }
}

/// A wrapper around existing iterator, will prevent users from calling `next` when the iterator is
/// invalid. If an iterator is already invalid, `next` does not do anything. If `next` returns an error,
/// `is_valid` should return false, and `next` should always return an error.
pub struct FusedIterator<I: StorageIterator> {
    iter: I,
    has_errored: bool,
}

impl<I: StorageIterator> FusedIterator<I> {
    pub fn new(iter: I) -> Self {
        Self {
            iter,
            has_errored: false,
        }
    }
}

impl<I: StorageIterator> StorageIterator for FusedIterator<I> {
    type KeyType<'a> = I::KeyType<'a> where Self: 'a;

    fn is_valid(&self) -> bool {
        !self.has_errored && self.iter.is_valid()
    }

    fn key(&self) -> Self::KeyType<'_> {
        assert!(self.is_valid());
        self.iter.key()
    }

    fn value(&self) -> &[u8] {
        assert!(self.is_valid());
        self.iter.value()
    }

    fn next(&mut self) -> Result<()> {
        if self.has_errored {
            return Err(anyhow::anyhow!("The iterator isn't valid"));
        }
        if self.iter.is_valid() {
            if let Err(e) = self.iter.next() {
                self.has_errored = true;
                return Err(e);
            }
        }
        Ok(())
    }
}
