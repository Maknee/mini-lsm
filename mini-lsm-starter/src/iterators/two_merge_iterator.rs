#![allow(unused_variables)] // TODO(you): remove this lint after implementing this mod
#![allow(dead_code)] // TODO(you): remove this lint after implementing this mod

use anyhow::Result;

use super::StorageIterator;

/// Merges two iterators of different types into one. If the two iterators have the same key, only
/// produce the key once and prefer the entry from A.
pub struct TwoMergeIterator<A: StorageIterator, B: StorageIterator> {
    a: A,
    b: B,
    // Add fields as need
    select_a: bool,
}

impl<
        A: 'static + StorageIterator,
        B: 'static + for<'a> StorageIterator<KeyType<'a> = A::KeyType<'a>>,
    > TwoMergeIterator<A, B>
{
    pub fn create(a: A, b: B) -> Result<Self> {
        let mut iter = Self {
            a,
            b,
            select_a: true,
        };
        iter.select_a_check();
        Ok(iter)
    }

    pub fn select_a_check(&mut self) {
        if self.a.is_valid() && self.b.is_valid() {
            self.select_a = self.a.key() <= self.b.key();
        } else if !self.a.is_valid() {
            self.select_a = false;
        } else {
            self.select_a = true;
        }
    }
}

impl<
        A: 'static + StorageIterator,
        B: 'static + for<'a> StorageIterator<KeyType<'a> = A::KeyType<'a>>,
    > StorageIterator for TwoMergeIterator<A, B>
{
    type KeyType<'a> = A::KeyType<'a>;

    fn key(&self) -> Self::KeyType<'_> {
        if self.select_a {
            self.a.key()
        } else {
            self.b.key()
        }
    }

    fn value(&self) -> &[u8] {
        if self.select_a {
            self.a.value()
        } else {
            self.b.value()
        }
    }

    fn is_valid(&self) -> bool {
        self.a.is_valid() || self.b.is_valid()
    }

    fn next(&mut self) -> Result<()> {
        if !self.a.is_valid() {
            self.select_a = false;
            self.b.next()?;
            return Ok(());
        }
        if !self.b.is_valid() {
            self.select_a = true;
            self.a.next()?;
            return Ok(());
        }

        if self.a.key() < self.b.key() {
            self.a.next()?;
        } else if self.a.key() == self.b.key() {
            self.a.next()?;
            self.b.next()?;
        } else {
            self.b.next()?;
        }
        self.select_a_check();
        Ok(())
    }

    fn num_active_iterators(&self) -> usize {
        self.a.num_active_iterators() + self.b.num_active_iterators()
    }
}
