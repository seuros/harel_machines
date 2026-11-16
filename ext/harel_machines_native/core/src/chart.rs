//! The state tree: what a chart is built from, and what it answers about
//! itself.

use crate::buffer::broken;
use crate::history::History;
use crate::{ChartError, Configuration, MAX_STATES, Refusal, StateId, validate};

/// What a state's children mean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Composition {
    /// A leaf. Entry stops here, and nothing may name it as a parent.
    #[default]
    Atomic,
    /// One child is active at a time.
    Compound {
        /// The child entered when nothing else selects one. It must be a
        /// child of this state.
        initial: StateId,
    },
    /// Every child is a region and all of them are active at once. It needs
    /// at least one region.
    Parallel,
}

/// A state's place in the tree, as declared.
///
/// The index of a state in the slice a [`Chart`] is built from is its
/// identity; this says only who contains it, what its children mean, and what
/// entering it again restores. A `State` on its own is not checked: the chart
/// checks the whole slice when it is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State {
    /// The containing state, or `None` for the root.
    pub parent: Option<StateId>,
    /// What the state's children mean.
    pub composition: Composition,
    /// What entering the state again restores.
    pub history: History,
}

impl State {
    /// A leaf.
    #[must_use]
    pub const fn atomic(parent: Option<StateId>) -> Self {
        Self {
            parent,
            composition: Composition::Atomic,
            history: History::None,
        }
    }

    /// A state with one child active at a time, entered at `initial` unless
    /// history or the target itself selects another child.
    #[must_use]
    pub const fn compound(parent: Option<StateId>, initial: StateId) -> Self {
        Self {
            parent,
            composition: Composition::Compound { initial },
            history: History::None,
        }
    }

    /// A state whose children are regions, all active at once.
    #[must_use]
    pub const fn parallel(parent: Option<StateId>) -> Self {
        Self {
            parent,
            composition: Composition::Parallel,
            history: History::None,
        }
    }

    /// Marks the state so entering it again restores what it last held.
    #[must_use]
    pub const fn with_history(self, history: History) -> Self {
        Self { history, ..self }
    }
}

/// One tree of at most `STATES` states, addressed by [`StateId`].
///
/// A chart is checked when it is built ([`Chart::new`]), so every chart that
/// exists is a single tree whose states are all reachable: one root, no
/// cycle, every compound state's initial child its own, every parallel state
/// with a region, no atomic state with children. Every call that reads a
/// state index refuses one past the last state with
/// [`Refusal::UnknownState`].
///
/// The chart is fixed-size and holds no heap. Its scratch space during a
/// call is a few arrays of `STATES` entries on the stack, and every walk is
/// iterative, so the stack a call needs grows with `STATES`, not with the
/// depth of the tree. Measured for `Chart<MAX_STATES>` (aarch64, Rust 1.99,
/// the least thread stack each call completes on, the call alone): release
/// builds need 1.75 MiB for [`Chart::new`] (which holds the chart it
/// returns), 0.81 MiB for [`Chart::configuration`], 2.25 MiB for
/// [`Chart::enter`] and 2.37 MiB for [`Chart::resolve`]; debug builds need
/// 4.37, 1.31, 3.25 and 3.94 MiB. A smaller `STATES` needs proportionally
/// less. Size a thread from these, with headroom for its own frames.
///
/// ```
/// use harel_machines::{Chart, State, TransitionKind};
///
/// // 0 Player
/// // |- 1 Stopped
/// // `- 2 Playing
/// //    |- 3 Buffering
/// //    `- 4 Streaming
/// let player = Chart::<5>::new(&[
///     State::compound(None, 1),
///     State::atomic(Some(0)),
///     State::compound(Some(0), 3),
///     State::atomic(Some(2)),
///     State::atomic(Some(2)),
/// ])?;
///
/// let start = player.enter(0)?;
/// assert_eq!(start.entry(), &[0, 1]);
/// assert_eq!(start.configuration().as_slice(), &[1]);
///
/// // Entering a composite state descends to a leaf.
/// let play = player.resolve(start.configuration(), 1, 2, TransitionKind::External)?;
/// assert_eq!(play.entry(), &[2, 3]);
///
/// // Leaving one exits every state it contains, innermost first.
/// let stop = player.resolve(play.configuration(), 3, 1, TransitionKind::External)?;
/// assert_eq!(stop.exit(), &[3, 2]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chart<const STATES: usize> {
    states: [State; STATES],
    depths: [StateId; STATES],
    children: [StateId; STATES],
    len: StateId,
    root: StateId,
}

impl<const STATES: usize> Chart<STATES> {
    /// A chart of `states`, the index of each in the slice being its
    /// [`StateId`]. The slice may be shorter than `STATES`; the rest of the
    /// capacity holds nothing.
    ///
    /// A `STATES` above [`MAX_STATES`] does not compile:
    ///
    /// ```compile_fail
    /// let _ = harel_machines::Chart::<65_536>::new(&[]);
    /// ```
    ///
    /// # Errors
    ///
    /// The first [`ChartError`], in the order of its variants, the states
    /// break.
    pub fn new(states: &[State]) -> Result<Self, ChartError> {
        const {
            assert!(
                STATES <= MAX_STATES,
                "a chart holds at most MAX_STATES states"
            );
        }
        let len = match states.len() {
            0 => return Err(ChartError::Empty),
            len if len > STATES => return Err(ChartError::TooMany),
            len => StateId::try_from(len).map_err(|_| ChartError::TooMany)?,
        };
        let mut chart = Self {
            states: [State::atomic(None); STATES],
            depths: [0; STATES],
            children: [0; STATES],
            len,
            root: 0,
        };
        for (cell, state) in chart.states.iter_mut().zip(states) {
            *cell = *state;
        }
        chart.root = validate::tree(states, &mut chart.depths, &mut chart.children)?;
        Ok(chart)
    }

    /// How many states the chart holds.
    #[must_use]
    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// Always `false`: a chart holds at least one state.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// The states as declared, in index order.
    #[must_use]
    pub fn states(&self) -> &[State] {
        self.states.get(..self.len()).unwrap_or(&[])
    }

    /// The state without a parent, which contains every other.
    #[must_use]
    pub const fn root(&self) -> StateId {
        self.root
    }

    /// The state at `id`.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownState`] for an index past the last state.
    pub fn state(&self, id: StateId) -> Result<State, Refusal> {
        self.states()
            .get(usize::from(id))
            .copied()
            .ok_or(Refusal::UnknownState)
    }

    /// The state containing `id`, or `None` for the root.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownState`] for an index past the last state.
    pub fn parent(&self, id: StateId) -> Result<Option<StateId>, Refusal> {
        self.state(id).map(|state| state.parent)
    }

    /// How many states contain this one. The root is at depth zero.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownState`] for an index past the last state.
    pub fn depth(&self, id: StateId) -> Result<usize, Refusal> {
        self.state(id)?;
        self.depths
            .get(usize::from(id))
            .map(|&depth| usize::from(depth))
            .ok_or(Refusal::UnknownState)
    }

    /// The parent of a state known not to be the root: one at a depth above
    /// zero, or one on the way up to a state it is known to be below.
    pub(crate) fn up(&self, id: StateId) -> Result<StateId, Refusal> {
        self.parent(id)?
            .ok_or_else(|| broken("a walk up the tree went past the root"))
    }

    /// How many children `id` has: its regions, for a parallel state.
    pub(crate) fn children(&self, id: StateId) -> StateId {
        self.children.get(usize::from(id)).copied().unwrap_or(0)
    }

    /// Whether `ancestor` contains `descendant`. A state does not contain
    /// itself.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownState`] if either index is past the last state.
    pub fn is_ancestor(&self, ancestor: StateId, descendant: StateId) -> Result<bool, Refusal> {
        let (above, below) = (self.depth(ancestor)?, self.depth(descendant)?);
        if above >= below {
            return Ok(false);
        }
        let mut cursor = descendant;
        for _ in above..below {
            cursor = self.up(cursor)?;
        }
        Ok(cursor == ancestor)
    }

    /// The innermost state that contains or is both `left` and `right`.
    ///
    /// This is what bounds an exit and entry set: no state above it is
    /// touched. The least common ancestor of a state and itself is that state,
    /// and of a state and one it contains is the containing state. A chart is
    /// one tree, so there always is one.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownState`] if either index is past the last state.
    pub fn least_common_ancestor(&self, left: StateId, right: StateId) -> Result<StateId, Refusal> {
        let (mut lower, mut upper) = (left, right);
        let (mut lower_depth, mut upper_depth) = (self.depth(lower)?, self.depth(upper)?);
        while lower_depth > upper_depth {
            lower = self.up(lower)?;
            lower_depth -= 1;
        }
        while upper_depth > lower_depth {
            upper = self.up(upper)?;
            upper_depth -= 1;
        }
        while lower != upper {
            lower = self.up(lower)?;
            upper = self.up(upper)?;
        }
        Ok(lower)
    }

    /// The configuration whose leaves are `leaves`, in any order, checked
    /// against this chart: what restores one saved elsewhere.
    ///
    /// It accepts exactly the configurations the chart can be in: none at
    /// all (a chart not entered yet), or atomic leaves, each named once,
    /// such that every compound state they put in the active set has one
    /// active child and every parallel state there has all of its regions.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownState`] for an index past the last state,
    /// [`Refusal::InvalidConfiguration`] for anything else the chart cannot
    /// be in.
    pub fn configuration(&self, leaves: &[StateId]) -> Result<Configuration<STATES>, Refusal> {
        validate::leaves(self, leaves)
    }
}
