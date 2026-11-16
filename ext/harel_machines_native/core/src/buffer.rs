//! A fixed-capacity ordered list of state indices, and the one answer for a
//! broken invariant.

use crate::{Refusal, StateId};

/// What a call answers when the core's own invariants are broken.
///
/// A built chart is a tree, and the configuration and history a call reads
/// are checked against it first, so nothing a caller passes reaches here: a
/// transition exits and enters each state at most once, and every index it
/// reads is one of the chart's. Tests stop on it; a release build refuses
/// with nothing changed rather than panic.
pub fn broken(invariant: &str) -> Refusal {
    debug_assert!(false, "broken invariant: {invariant}");
    Refusal::InvalidConfiguration
}

/// Holds an exit or an entry set without a heap.
///
/// A transition enters and exits each state at most once, so a capacity
/// equal to the chart's cannot be exceeded.
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

    /// Appends `id`; a full buffer is a broken invariant.
    pub fn push(&mut self, id: StateId) -> Result<(), Refusal> {
        let slot = self
            .ids
            .get_mut(self.len)
            .ok_or_else(|| broken("a transition touches a state twice"))?;
        *slot = id;
        self.len += 1;
        Ok(())
    }

    pub fn as_slice(&self) -> &[StateId] {
        self.ids.get(..self.len).unwrap_or(&[])
    }

    pub fn as_mut_slice(&mut self) -> &mut [StateId] {
        self.ids.get_mut(..self.len).unwrap_or(&mut [])
    }
}

impl<const N: usize> PartialEq for Buffer<N> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}
