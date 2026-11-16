//! The chart behind a runtime state count: the core at the smallest of the
//! [`SHAPES`] that holds it, behind one object-safe trait over plain slices.
use harel_machines::{
    Chart, ChartError, LastActive, Refusal, State, StateId, Transition, TransitionKind,
};

/// State counts a chart is monomorphised for.
pub const SHAPES: [usize; 4] = [8, 32, 128, 512];

/// A child each compound state last held: (compound state, child).
pub type Remembered = Vec<(StateId, StateId)>;

/// A resolved transition as state indices on the heap.
pub struct Resolved {
    pub exit: Vec<StateId>,
    pub entry: Vec<StateId>,
    pub configuration: Vec<StateId>,
}

impl<const N: usize> From<&Transition<N>> for Resolved {
    fn from(transition: &Transition<N>) -> Self {
        Self {
            exit: transition.exit().to_vec(),
            entry: transition.entry().to_vec(),
            configuration: transition.configuration().as_slice().to_vec(),
        }
    }
}

/// What the binding asks of a chart, whatever its capacity. Configurations
/// and histories cross as slices and are rebuilt through the core's restoring
/// constructors on every call, so the core checks them each time.
pub trait Shape: Send + Sync {
    fn root(&self) -> StateId;
    fn parent(&self, id: StateId) -> Result<Option<StateId>, Refusal>;
    fn depth(&self, id: StateId) -> Result<usize, Refusal>;
    fn is_ancestor(&self, ancestor: StateId, descendant: StateId) -> Result<bool, Refusal>;
    fn least_common_ancestor(&self, left: StateId, right: StateId) -> Result<StateId, Refusal>;
    fn configuration(&self, leaves: &[StateId]) -> Result<Vec<StateId>, Refusal>;
    fn enter(
        &self,
        target: StateId,
        remembered: &[(StateId, StateId)],
    ) -> Result<Resolved, Refusal>;
    fn resolve(
        &self,
        leaves: &[StateId],
        source: StateId,
        target: StateId,
        kind: TransitionKind,
        remembered: &[(StateId, StateId)],
    ) -> Result<Resolved, Refusal>;
    fn restored(&self, remembered: &[(StateId, StateId)]) -> Result<Remembered, Refusal>;
    fn record(
        &self,
        remembered: &[(StateId, StateId)],
        exited: &[StateId],
    ) -> Result<Remembered, Refusal>;
}

impl<const N: usize> Shape for Chart<N> {
    fn root(&self) -> StateId {
        Self::root(self)
    }

    fn parent(&self, id: StateId) -> Result<Option<StateId>, Refusal> {
        Self::parent(self, id)
    }

    fn depth(&self, id: StateId) -> Result<usize, Refusal> {
        Self::depth(self, id)
    }

    fn is_ancestor(&self, ancestor: StateId, descendant: StateId) -> Result<bool, Refusal> {
        Self::is_ancestor(self, ancestor, descendant)
    }

    fn least_common_ancestor(&self, left: StateId, right: StateId) -> Result<StateId, Refusal> {
        Self::least_common_ancestor(self, left, right)
    }

    fn configuration(&self, leaves: &[StateId]) -> Result<Vec<StateId>, Refusal> {
        Self::configuration(self, leaves).map(|held| held.as_slice().to_vec())
    }

    fn enter(
        &self,
        target: StateId,
        remembered: &[(StateId, StateId)],
    ) -> Result<Resolved, Refusal> {
        let history = LastActive::restored(self, remembered)?;
        Ok(Resolved::from(&self.enter_with_history(target, &history)?))
    }

    fn resolve(
        &self,
        leaves: &[StateId],
        source: StateId,
        target: StateId,
        kind: TransitionKind,
        remembered: &[(StateId, StateId)],
    ) -> Result<Resolved, Refusal> {
        let configuration = Self::configuration(self, leaves)?;
        let history = LastActive::restored(self, remembered)?;
        let step = self.resolve_with_history(&configuration, source, target, kind, &history)?;
        Ok(Resolved::from(&step))
    }

    fn restored(&self, remembered: &[(StateId, StateId)]) -> Result<Remembered, Refusal> {
        LastActive::restored(self, remembered).map(|history| history.remembered().collect())
    }

    fn record(
        &self,
        remembered: &[(StateId, StateId)],
        exited: &[StateId],
    ) -> Result<Remembered, Refusal> {
        let mut history = LastActive::restored(self, remembered)?;
        history.record(self, exited)?;
        Ok(history.remembered().collect())
    }
}

/// Why a chart was not built.
pub enum Unbuilt {
    /// More states than the largest shape holds.
    Unsupported,
    /// The core refused the states.
    Refused(ChartError),
}

/// The chart of `states` at the smallest shape that holds them.
pub fn build(states: &[State]) -> Result<Box<dyn Shape>, Unbuilt> {
    fn boxed<const N: usize>(states: &[State]) -> Result<Box<dyn Shape>, Unbuilt> {
        let chart = Box::new(Chart::<N>::new(states).map_err(Unbuilt::Refused)?);
        Ok(chart)
    }
    match states.len() {
        0..=8 => boxed::<8>(states),
        9..=32 => boxed::<32>(states),
        33..=128 => boxed::<128>(states),
        129..=512 => boxed::<512>(states),
        _ => Err(Unbuilt::Unsupported),
    }
}
