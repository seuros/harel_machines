//! The set of leaves a chart is in.

use crate::StateId;

/// The active leaves of a [`Chart`](crate::Chart), in ascending index order.
///
/// A flat machine is in one state. A chart whose active branch crosses a
/// parallel state is in one leaf per region, so the configuration is a set.
///
/// A configuration comes from a chart: [`Configuration::new`] is the one
/// before anything is entered, a [`Transition`](crate::Transition) carries
/// the one it settles in, and [`Chart::configuration`](crate::Chart::configuration)
/// checks one restored from elsewhere. It has no other constructor and no
/// mutator, so every configuration is one some chart can be in.
///
/// ```
/// use harel_machines::{Chart, Configuration, State};
///
/// let chart = Chart::<3>::new(&[
///     State::parallel(None),
///     State::atomic(Some(0)),
///     State::atomic(Some(0)),
/// ])?;
/// let configuration = chart.configuration(&[2, 1])?;
///
/// assert_eq!(configuration.as_slice(), &[1, 2]);
/// assert!(configuration.contains(1));
/// assert!(Configuration::<3>::new().is_empty());
/// # Ok::<(), Box<dyn std::error::Error>>(())
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

    /// The leaves marked in `leaf`, by index, in ascending order. The caller
    /// has checked they form a configuration of its chart.
    pub(crate) fn from_marks(leaf: &[bool]) -> Self {
        let mut configuration = Self::new();
        for (id, _) in crate::indexed(leaf).filter(|(_, marked)| **marked) {
            if let Some(slot) = configuration.leaves.get_mut(configuration.len) {
                *slot = id;
                configuration.len += 1;
            }
        }
        configuration
    }

    /// Whether `leaf` is one of the active leaves.
    #[must_use]
    pub fn contains(&self, leaf: StateId) -> bool {
        self.as_slice().binary_search(&leaf).is_ok()
    }

    /// How many leaves are active.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether no leaf is active: the chart has not been entered.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The active leaves, in ascending index order.
    #[must_use]
    pub fn as_slice(&self) -> &[StateId] {
        self.leaves.get(..self.len).unwrap_or(&[])
    }

    /// The active leaves, in ascending index order.
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
