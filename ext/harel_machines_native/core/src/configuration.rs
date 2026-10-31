//! The set of leaves a chart is in.

use crate::StateId;

/// The active leaves of a [`Chart`](crate::Chart), in ascending index order.
///
/// A flat machine is in one state. A chart whose active branch crosses a
/// parallel state is in one leaf per region, so the configuration is a set.
///
/// ```
/// use harel_machines::Configuration;
///
/// let mut configuration = Configuration::<4>::new();
/// configuration.insert(3);
/// configuration.insert(1);
/// configuration.insert(3);
///
/// assert_eq!(configuration.as_slice(), &[1, 3]);
/// assert!(configuration.contains(1));
/// ```
#[derive(Debug, Clone, Copy, Eq)]
pub struct Configuration<const STATES: usize> {
    leaves: [StateId; STATES],
    len: usize,
}

impl<const STATES: usize> Configuration<STATES> {
    /// A configuration holding nothing, which is a chart that has not been
    /// entered.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            leaves: [0; STATES],
            len: 0,
        }
    }

    /// Adds a leaf. Already being in it changes nothing.
    pub const fn insert(&mut self, leaf: StateId) {
        let mut index = 0;
        while index < self.len {
            if self.leaves[index] == leaf {
                return;
            }
            if self.leaves[index] > leaf {
                break;
            }
            index += 1;
        }
        let mut tail = self.len;
        while tail > index {
            self.leaves[tail] = self.leaves[tail - 1];
            tail -= 1;
        }
        self.leaves[index] = leaf;
        self.len += 1;
    }

    /// Drops a leaf. Not being in it changes nothing.
    pub const fn remove(&mut self, leaf: StateId) {
        let mut index = 0;
        while index < self.len {
            if self.leaves[index] == leaf {
                let mut shift = index;
                while shift + 1 < self.len {
                    self.leaves[shift] = self.leaves[shift + 1];
                    shift += 1;
                }
                self.len -= 1;
                return;
            }
            index += 1;
        }
    }

    #[must_use]
    pub fn contains(&self, leaf: StateId) -> bool {
        self.as_slice().contains(&leaf)
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[must_use]
    pub fn as_slice(&self) -> &[StateId] {
        &self.leaves[..self.len]
    }

    pub fn iter(&self) -> core::slice::Iter<'_, StateId> {
        self.as_slice().iter()
    }
}

impl<const STATES: usize> Default for Configuration<STATES> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const STATES: usize> PartialEq for Configuration<STATES> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<'a, const STATES: usize> IntoIterator for &'a Configuration<STATES> {
    type Item = &'a StateId;
    type IntoIter = core::slice::Iter<'a, StateId>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests;
