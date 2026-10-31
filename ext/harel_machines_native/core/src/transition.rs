//! What resolving a transition over the tree comes out as.

use crate::StateId;
use crate::buffer::Buffer;
use crate::configuration::Configuration;

/// Whether a transition leaves the state it starts from.
///
/// The distinction only bites when the source contains the target or is the
/// target. Everywhere else the two kinds resolve alike.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransitionKind {
    /// The source is exited and entered again.
    #[default]
    External,
    /// The source stays active, and so does everything between it and the
    /// target.
    Internal,
}

/// The states a transition leaves and joins.
///
/// `exit` runs innermost first, so a child is left before the parent that
/// contains it. `entry` runs outermost first, so a parent is joined before the
/// child it contains. `configuration` is the active leaf set that results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition<const STATES: usize> {
    exit: Buffer<STATES>,
    entry: Buffer<STATES>,
    configuration: Configuration<STATES>,
}

impl<const STATES: usize> Transition<STATES> {
    pub(crate) const fn new(
        exit: Buffer<STATES>,
        entry: Buffer<STATES>,
        configuration: Configuration<STATES>,
    ) -> Self {
        Self {
            exit,
            entry,
            configuration,
        }
    }

    /// The states to leave, innermost first. States at the same depth leave in
    /// the order of the active leaf that put them here.
    #[must_use]
    pub fn exit(&self) -> &[StateId] {
        self.exit.as_slice()
    }

    /// The states to join, outermost first, ending at a leaf in every region
    /// entered.
    #[must_use]
    pub fn entry(&self) -> &[StateId] {
        self.entry.as_slice()
    }

    /// The active leaves once this transition has been taken.
    #[must_use]
    pub const fn configuration(&self) -> &Configuration<STATES> {
        &self.configuration
    }
}
