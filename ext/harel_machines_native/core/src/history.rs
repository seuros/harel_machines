//! What re-entering a composite state restores.

use crate::StateId;
use crate::chart::Chart;
use crate::transition::Transition;

/// What a composite state restores when it is entered again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum History {
    /// Nothing. Entry goes to the initial child.
    #[default]
    None,
    /// The child that was active when the state was last exited. Below that
    /// child, entry follows initial children again.
    Shallow,
    /// The whole configuration that was active when the state was last exited.
    Deep,
}

/// The child each composite state held when it was last exited.
///
/// One slot per state is all history needs: shallow history reads the slot of
/// the state being entered, and deep history keeps reading slots on the way
/// down, which rebuilds the last configuration without storing a set per state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LastActive<const STATES: usize> {
    children: [Option<StateId>; STATES],
}

impl<const STATES: usize> LastActive<STATES> {
    /// Remembers nothing, so every history state falls back to its initial
    /// child.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            children: [None; STATES],
        }
    }

    /// Files every state the transition exited under its parent.
    pub fn record(&mut self, chart: &Chart<STATES>, transition: &Transition<STATES>) {
        for &exited in transition.exit() {
            if let Some(parent) = chart.parent(exited)
                && let Some(slot) = self.children.get_mut(usize::from(parent))
            {
                *slot = Some(exited);
            }
        }
    }

    /// Files a child by hand, for a machine whose history was persisted
    /// elsewhere. A parent outside this chart records nothing.
    pub fn remember(&mut self, parent: StateId, child: StateId) {
        if let Some(slot) = self.children.get_mut(usize::from(parent)) {
            *slot = Some(child);
        }
    }

    /// The child a state held when it was last exited, or `None` for a state
    /// outside this chart.
    #[must_use]
    pub fn last_child(&self, state: StateId) -> Option<StateId> {
        self.children.get(usize::from(state)).copied().flatten()
    }
}

impl<const STATES: usize> Default for LastActive<STATES> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
