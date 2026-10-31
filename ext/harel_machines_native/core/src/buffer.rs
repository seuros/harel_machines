//! A fixed-capacity ordered list of state indices.

use crate::{Error, StateId};

/// Holds an exit or an entry set without a heap.
///
/// A transition enters and exits each state at most once, so a capacity equal
/// to the chart's state count cannot be exceeded by a tree.
#[derive(Debug, Clone, Copy, Eq)]
pub struct Buffer<const N: usize> {
    ids: [StateId; N],
    len: usize,
}

impl<const N: usize> Buffer<N> {
    pub const fn new() -> Self {
        Self {
            ids: [0; N],
            len: 0,
        }
    }

    /// Fails with [`Error::NotATree`] once more states have been pushed than
    /// the chart holds.
    pub const fn push(&mut self, id: StateId) -> Result<(), Error> {
        if self.len == N {
            return Err(Error::NotATree);
        }
        self.ids[self.len] = id;
        self.len += 1;
        Ok(())
    }

    pub fn as_slice(&self) -> &[StateId] {
        &self.ids[..self.len]
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub fn contains(&self, id: StateId) -> bool {
        self.as_slice().contains(&id)
    }

    pub const fn get(&self, index: usize) -> StateId {
        self.ids[index]
    }

    pub const fn swap(&mut self, left: usize, right: usize) {
        self.ids.swap(left, right);
    }
}

impl<const N: usize> PartialEq for Buffer<N> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}
