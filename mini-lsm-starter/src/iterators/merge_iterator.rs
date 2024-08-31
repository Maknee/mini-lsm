#![allow(unused_variables)] // TODO(you): remove this lint after implementing this mod
#![allow(dead_code)] // TODO(you): remove this lint after implementing this mod

use std::cmp::{self};
use std::collections::binary_heap::PeekMut;
use std::collections::BinaryHeap;

use anyhow::Result;

use crate::key::KeySlice;

use super::StorageIterator;

struct HeapWrapper<I: StorageIterator>(pub usize, pub Box<I>);

impl<I: StorageIterator> PartialEq for HeapWrapper<I> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == cmp::Ordering::Equal
    }
}

impl<I: StorageIterator> Eq for HeapWrapper<I> {}

impl<I: StorageIterator> PartialOrd for HeapWrapper<I> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<I: StorageIterator> Ord for HeapWrapper<I> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.1
            .key()
            .cmp(&other.1.key())
            .then(self.0.cmp(&other.0))
            .reverse()
    }
}

/// Merge multiple iterators of the same type. If the same key occurs multiple times in some
/// iterators, prefer the one with smaller index.
pub struct MergeIterator<I: StorageIterator> {
    iters: BinaryHeap<HeapWrapper<I>>,
    current: Option<HeapWrapper<I>>,
}

impl<I: StorageIterator> MergeIterator<I> {
    pub fn create(iters: Vec<Box<I>>) -> Self {
        let mut heap = BinaryHeap::new();
        for (i, x) in iters.into_iter().enumerate() {
            let h = HeapWrapper(i, x);
            if h.1.is_valid() {
                heap.push(h);
            }
        }
        let current = if !heap.is_empty() { heap.pop() } else { None };
        Self {
            iters: heap,
            current: current,
        }
    }
}

impl<I: 'static + for<'a> StorageIterator<KeyType<'a> = KeySlice<'a>>> StorageIterator
    for MergeIterator<I>
{
    type KeyType<'a> = KeySlice<'a>;

    fn key(&self) -> KeySlice {
        if let Some(x) = &self.current {
            x.1.key()
        } else {
            panic!("No current key");
        }
    }

    fn value(&self) -> &[u8] {
        if let Some(x) = &self.current {
            x.1.value()
        } else {
            panic!("No current value");
        }
    }

    fn is_valid(&self) -> bool {
        if let Some(x) = &self.current {
            x.1.is_valid()
        } else {
            false
        }
    }

    fn next(&mut self) -> Result<()> {
        let current = self.current.as_mut().unwrap();

        // Handle duplicates
        while let Some(mut inner) = self.iters.peek_mut() {
            // same key as current key, we need to go to the next one
            if current.1.key() == inner.1.key() {
                let next_key = inner.1.next();
                // case 1: iterator not good
                if let e @ Err(_) = next_key {
                    PeekMut::pop(inner);
                    return e;
                }

                // case 2: iterator is invalid
                if !inner.1.is_valid() {
                    PeekMut::pop(inner);
                }
            } else {
                break;
            }
        }

        // advance next key
        current.1.next()?;

        // check if key is still valid
        if !current.1.is_valid() {
            // pop the next iter
            if let Some(iter) = self.iters.pop() {
                *current = iter;
            }
            return Ok(());
        }

        // compare and swap with next iterator is valid
        if let Some(mut inner) = self.iters.peek_mut() {
            if *current < *inner {
                std::mem::swap(&mut *inner, current);
            }
        }

        Ok(())
    }
}
