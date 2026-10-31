//! Dispatch from a runtime state count to the shapes the core is monomorphised for.

use harel_machines::{
    Chart, Configuration, Error, LastActive, State, StateId, Transition, TransitionKind,
};

/// State counts [`Tree`] is monomorphised for.
pub const SHAPES: [usize; 4] = [8, 32, 128, 512];

/// A chart at one of the [`SHAPES`].
pub enum Tree {
    S8(Box<Chart<8>>),
    S32(Box<Chart<32>>),
    S128(Box<Chart<128>>),
    S512(Box<Chart<512>>),
}

/// Runs `body` against whichever variant is held.
macro_rules! tree {
    ($tree:expr, |$inner:ident| $body:expr) => {
        match $tree {
            Tree::S8($inner) => $body,
            Tree::S32($inner) => $body,
            Tree::S128($inner) => $body,
            Tree::S512($inner) => $body,
        }
    };
}

/// A resolved transition as state indices on the heap.
pub struct Resolved {
    pub exit: Vec<StateId>,
    pub entry: Vec<StateId>,
    pub configuration: Vec<StateId>,
}

impl<const STATES: usize> From<&Transition<STATES>> for Resolved {
    fn from(transition: &Transition<STATES>) -> Self {
        Self {
            exit: transition.exit().to_vec(),
            entry: transition.entry().to_vec(),
            configuration: transition.configuration().as_slice().to_vec(),
        }
    }
}

/// Constructs one shape behind its own call frame.
#[inline(never)]
fn boxed<const STATES: usize>(states: &[State]) -> Box<Chart<STATES>> {
    let mut cells = [State::atomic(None); STATES];
    for (cell, state) in cells.iter_mut().zip(states) {
        *cell = *state;
    }
    Box::new(Chart::new(cells))
}

/// The slots this shape holds, as what the core reads history from.
#[inline(never)]
fn recorded<const STATES: usize>(children: &[Option<StateId>]) -> LastActive<STATES> {
    let mut last_active = LastActive::new();
    for (parent, child) in (0..).zip(children.iter().take(STATES)) {
        if let Some(child) = *child {
            last_active.remember(parent, child);
        }
    }
    last_active
}

/// Entering a state from nothing, under this shape.
#[inline(never)]
fn entered<const STATES: usize>(
    chart: &Chart<STATES>,
    state: StateId,
    children: &[Option<StateId>],
) -> Result<Resolved, Error> {
    let transition = chart.enter_with_history(state, &recorded(children))?;
    Ok(Resolved::from(&transition))
}

/// Resolving a transition over the configuration, under this shape.
#[inline(never)]
fn resolved<const STATES: usize>(
    chart: &Chart<STATES>,
    leaves: &[StateId],
    source: StateId,
    target: StateId,
    kind: TransitionKind,
    children: &[Option<StateId>],
) -> Result<Resolved, Error> {
    let mut configuration = Configuration::new();
    for &leaf in leaves {
        configuration.insert(leaf);
    }
    let transition =
        chart.resolve_with_history(&configuration, source, target, kind, &recorded(children))?;
    Ok(Resolved::from(&transition))
}

impl Tree {
    /// A chart of `states`, or `None` when there are more of them than the
    /// largest shape holds.
    pub fn new(states: &[State]) -> Option<Self> {
        match states.len() {
            count if count <= 8 => Some(Self::S8(boxed(states))),
            count if count <= 32 => Some(Self::S32(boxed(states))),
            count if count <= 128 => Some(Self::S128(boxed(states))),
            count if count <= 512 => Some(Self::S512(boxed(states))),
            _ => None,
        }
    }

    pub fn parent(&self, state: StateId) -> Option<StateId> {
        tree!(self, |chart| chart.parent(state))
    }

    /// # Errors
    ///
    /// As [`Chart::depth`].
    pub fn depth(&self, state: StateId) -> Result<usize, Error> {
        tree!(self, |chart| chart.depth(state))
    }

    /// # Errors
    ///
    /// As [`Chart::is_ancestor`].
    pub fn ancestor(&self, ancestor: StateId, descendant: StateId) -> Result<bool, Error> {
        tree!(self, |chart| chart.is_ancestor(ancestor, descendant))
    }

    /// # Errors
    ///
    /// As [`Chart::least_common_ancestor`].
    pub fn least_common_ancestor(&self, left: StateId, right: StateId) -> Result<StateId, Error> {
        tree!(self, |chart| chart.least_common_ancestor(left, right))
    }

    /// # Errors
    ///
    /// As [`Chart::enter_with_history`].
    pub fn enter(&self, state: StateId, children: &[Option<StateId>]) -> Result<Resolved, Error> {
        tree!(self, |chart| entered(chart, state, children))
    }

    /// # Errors
    ///
    /// As [`Chart::resolve_with_history`].
    pub fn resolve(
        &self,
        leaves: &[StateId],
        source: StateId,
        target: StateId,
        kind: TransitionKind,
        children: &[Option<StateId>],
    ) -> Result<Resolved, Error> {
        tree!(self, |chart| resolved(
            chart, leaves, source, target, kind, children
        ))
    }
}
