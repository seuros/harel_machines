//! What re-entering a compound state restores.

use crate::{Chart, Composition, Refusal, StateId, validate};

/// What a composite state restores when it is entered again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum History {
    /// Nothing. Entry goes to the initial child.
    #[default]
    None,
    /// The child that was active when the state was last exited. Below that
    /// child, entry follows initial children again.
    Shallow,
    /// The whole configuration that was active when the state was last
    /// exited. On a parallel state it reaches into every region.
    Deep,
}

/// The child each compound state held when it was last exited.
///
/// One slot per state is all history needs: shallow history reads the slot of
/// the state being entered, and deep history keeps reading slots on the way
/// down, which rebuilds the last configuration without storing a set per
/// state. Only a compound state has a slot that is ever filled: a parallel
/// state re-enters every region whatever was exited.
///
/// It only ever holds what some sequence of transitions over the chart it
/// was filled against leaves: both ways to fill one, [`LastActive::record`]
/// and [`LastActive::restored`], check that, and a chart resolving with
/// history checks it again.
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

    /// The history holding exactly `remembered`, pairs of a compound state
    /// and the child it last held, in any order: what restores one saved
    /// from [`LastActive::remembered`].
    ///
    /// It accepts exactly the histories some sequence of transitions over
    /// `chart`, each [`record`](LastActive::record)ed, can leave. A state is
    /// exited with everything active below it, so a remembered child that is
    /// itself compound remembers a child too, and a remembered parallel state
    /// has every compound region remembering one, all the way down.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownState`] for an index past the last state of
    /// `chart`, [`Refusal::InvalidHistory`] for a parent that is not a
    /// compound state containing its child, a parent named twice, or a
    /// history no transitions leave.
    pub fn restored(
        chart: &Chart<STATES>,
        remembered: &[(StateId, StateId)],
    ) -> Result<Self, Refusal> {
        let mut history = Self::new();
        for &(parent, child) in remembered {
            chart.state(parent)?;
            chart.state(child)?;
            let slot = history
                .children
                .get_mut(usize::from(parent))
                .ok_or(Refusal::UnknownState)?;
            if slot.replace(child).is_some() {
                return Err(Refusal::InvalidHistory);
            }
        }
        validate::history(chart, &history)?;
        Ok(history)
    }

    /// Every compound state that remembers a child, and that child, in
    /// ascending order of the compound state.
    pub fn remembered(&self) -> impl Iterator<Item = (StateId, StateId)> + '_ {
        crate::indexed(&self.children)
            .filter_map(|(parent, child)| child.map(|child| (parent, child)))
    }

    /// Files every state in `exited` (a transition's
    /// [`exit`](crate::Transition::exit) set) under its parent, where that
    /// parent is a compound state. A state whose parent is parallel, and the
    /// root, file nothing.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownState`] if any index is past the last state of
    /// `chart`. [`Refusal::InvalidHistory`] if `exited` is not exactly an
    /// exit set some transition of `chart` produces, in its order (a state
    /// named twice, two children of one compound state, a state without the
    /// active states below it, the wrong order), or if the history it would
    /// leave is one no transitions over `chart` leave (this history is
    /// another chart's, or the exit set contradicts it). Nothing is filed
    /// then.
    pub fn record(&mut self, chart: &Chart<STATES>, exited: &[StateId]) -> Result<(), Refusal> {
        validate::exits(chart, exited)?;
        let mut filed = *self;
        for &state in exited {
            if let Some(parent) = chart.parent(state)?
                && let Composition::Compound { .. } = chart.state(parent)?.composition
                && let Some(slot) = filed.children.get_mut(usize::from(parent))
            {
                *slot = Some(state);
            }
        }
        validate::history(chart, &filed)?;
        *self = filed;
        Ok(())
    }

    /// The child a state held when it was last exited, or `None` when it
    /// holds nothing or is past the end of the capacity.
    #[must_use]
    pub fn last_child(&self, state: StateId) -> Option<StateId> {
        self.children.get(usize::from(state)).copied().flatten()
    }

    /// Every slot, by state index.
    pub(crate) const fn slots(&self) -> &[Option<StateId>] {
        &self.children
    }
}

impl<const STATES: usize> Default for LastActive<STATES> {
    fn default() -> Self {
        Self::new()
    }
}
