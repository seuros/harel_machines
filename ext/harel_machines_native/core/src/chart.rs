//! The state tree, and the exit and entry sets a transition over it produces.

use crate::buffer::Buffer;
use crate::configuration::Configuration;
use crate::history::{History, LastActive};
use crate::transition::{Transition, TransitionKind};
use crate::{Error, StateId};

/// What a state's children mean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Composition {
    /// A leaf. Entry stops here.
    #[default]
    Atomic,
    /// One child is active at a time.
    Compound {
        /// The child entered when nothing else selects one.
        initial: StateId,
    },
    /// Every child is a region and all of them are active at once.
    Parallel,
}

/// A state's place in the tree.
///
/// The index of a state in the [`Chart`] is its identity; this says only who
/// contains it, what its children mean, and what entering it again restores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State {
    /// The containing state, or `None` for a root.
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

/// How far down an entry reads what a state last held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Restore {
    /// Open initial children.
    Initial,
    /// Open the remembered child at the next choice, then initial children.
    Child,
    /// Open remembered children all the way down.
    Subtree,
}

impl Restore {
    /// A state's own marker only ever widens what an enclosing one asked for.
    const fn upgraded_by(self, history: History) -> Self {
        match (self, history) {
            (Self::Subtree, _) | (_, History::Deep) => Self::Subtree,
            (Self::Child, _) | (_, History::Shallow) => Self::Child,
            (Self::Initial, History::None) => Self::Initial,
        }
    }
}

/// A tree of `STATES` states, addressed by [`StateId`].
///
/// Every state index must be below `STATES` and the parent links must form a
/// tree. A chart that breaks either is the caller's bug; where the break leaves
/// a question unanswerable, resolution returns [`Error::NotATree`].
///
/// ```
/// use harel_machines::{Chart, State, TransitionKind};
///
/// // 0 Player
/// // |- 1 Stopped
/// // `- 2 Playing
/// //    |- 3 Buffering
/// //    `- 4 Streaming
/// const PLAYER: Chart<5> = Chart::new([
///     State::compound(None, 1),
///     State::atomic(Some(0)),
///     State::compound(Some(0), 3),
///     State::atomic(Some(2)),
///     State::atomic(Some(2)),
/// ]);
///
/// let start = PLAYER.enter(0)?;
/// assert_eq!(start.entry(), &[0, 1]);
/// assert_eq!(start.configuration().as_slice(), &[1]);
///
/// // Entering a composite state descends to a leaf.
/// let play = PLAYER.resolve(start.configuration(), 1, 2, TransitionKind::External)?;
/// assert_eq!(play.entry(), &[2, 3]);
///
/// // Leaving one exits every state it contains, innermost first.
/// let stop = PLAYER.resolve(play.configuration(), 3, 1, TransitionKind::External)?;
/// assert_eq!(stop.exit(), &[3, 2]);
/// # Ok::<(), harel_machines::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chart<const STATES: usize> {
    states: [State; STATES],
}

impl<const STATES: usize> Chart<STATES> {
    #[must_use]
    pub const fn new(states: [State; STATES]) -> Self {
        Self { states }
    }

    #[must_use]
    pub fn state(&self, id: StateId) -> State {
        self.states[usize::from(id)]
    }

    #[must_use]
    pub fn parent(&self, id: StateId) -> Option<StateId> {
        self.states[usize::from(id)].parent
    }

    /// How many states contain this one. A root is at depth zero.
    ///
    /// # Errors
    ///
    /// [`Error::NotATree`] if the parent chain is longer than the chart has
    /// states.
    pub fn depth(&self, id: StateId) -> Result<usize, Error> {
        let mut depth = 0;
        let mut cursor = id;
        while let Some(parent) = self.parent(cursor) {
            depth += 1;
            if depth > STATES {
                return Err(Error::NotATree);
            }
            cursor = parent;
        }
        Ok(depth)
    }

    /// Whether `ancestor` contains `descendant`. A state does not contain
    /// itself.
    ///
    /// # Errors
    ///
    /// [`Error::NotATree`] if the parent chain is longer than the chart has
    /// states.
    pub fn is_ancestor(&self, ancestor: StateId, descendant: StateId) -> Result<bool, Error> {
        let mut steps = 0;
        let mut cursor = descendant;
        while let Some(parent) = self.parent(cursor) {
            if parent == ancestor {
                return Ok(true);
            }
            steps += 1;
            if steps > STATES {
                return Err(Error::NotATree);
            }
            cursor = parent;
        }
        Ok(false)
    }

    /// The innermost state that contains or is both `left` and `right`.
    ///
    /// This is what bounds an exit and entry set: no state at or above it is
    /// touched. The least common ancestor of a state and itself is that state,
    /// and of a state and one it contains is the containing state.
    ///
    /// # Errors
    ///
    /// [`Error::NoCommonAncestor`] if the two sit in disjoint trees.
    /// [`Error::NotATree`] if a parent chain is longer than the chart has
    /// states.
    pub fn least_common_ancestor(&self, left: StateId, right: StateId) -> Result<StateId, Error> {
        let mut lower = left;
        let mut lower_depth = self.depth(lower)?;
        let mut upper = right;
        let mut upper_depth = self.depth(upper)?;

        while lower_depth > upper_depth {
            lower = self.parent(lower).ok_or(Error::NotATree)?;
            lower_depth -= 1;
        }
        while upper_depth > lower_depth {
            upper = self.parent(upper).ok_or(Error::NotATree)?;
            upper_depth -= 1;
        }
        while lower != upper {
            lower = self.parent(lower).ok_or(Error::NoCommonAncestor)?;
            upper = self.parent(upper).ok_or(Error::NoCommonAncestor)?;
        }
        Ok(lower)
    }

    /// The entry set and configuration for entering `state` from nothing.
    ///
    /// # Errors
    ///
    /// [`Error::EmptyParallel`] if a parallel state on the way down has no
    /// regions. [`Error::NotATree`] if the descent visits more states than the
    /// chart holds.
    pub fn enter(&self, state: StateId) -> Result<Transition<STATES>, Error> {
        self.enter_with_history(state, &LastActive::new())
    }

    /// The entry set and configuration for entering `state` from nothing, with
    /// history states restored from `last_active`.
    ///
    /// # Errors
    ///
    /// As [`Chart::enter`].
    pub fn enter_with_history(
        &self,
        state: StateId,
        last_active: &LastActive<STATES>,
    ) -> Result<Transition<STATES>, Error> {
        let mut entry = Buffer::new();
        entry.push(state)?;
        self.descend(state, Restore::Initial, last_active, &mut entry)?;
        Ok(self.settle(Buffer::new(), entry, &Configuration::new()))
    }

    /// Which states a transition from `source` to `target` must exit and enter.
    ///
    /// What gets exited is read from `configuration`, which is what makes a
    /// parallel region's other leaves come along: `source` is expected to be
    /// active in it, or to contain something that is.
    ///
    /// Composite states on the way down are entered at their initial child; see
    /// [`Chart::resolve_with_history`] to restore what they last held instead.
    ///
    /// # Errors
    ///
    /// [`Error::NoCommonAncestor`] if source and target sit in disjoint trees,
    /// or an external transition leaves a root for itself.
    /// [`Error::EmptyParallel`] if a parallel state on the way down has no
    /// regions. [`Error::NotATree`] if a walk visits more states than the chart
    /// holds.
    pub fn resolve(
        &self,
        configuration: &Configuration<STATES>,
        source: StateId,
        target: StateId,
        kind: TransitionKind,
    ) -> Result<Transition<STATES>, Error> {
        self.resolve_with_history(configuration, source, target, kind, &LastActive::new())
    }

    /// Which states a transition from `source` to `target` must exit and enter,
    /// restoring history states from `last_active`.
    ///
    /// History only applies where entry has a child to choose: at the target
    /// and below it. The states named between the least common ancestor and
    /// the target are entered as named.
    ///
    /// # Errors
    ///
    /// As [`Chart::resolve`].
    pub fn resolve_with_history(
        &self,
        configuration: &Configuration<STATES>,
        source: StateId,
        target: StateId,
        kind: TransitionKind,
        last_active: &LastActive<STATES>,
    ) -> Result<Transition<STATES>, Error> {
        let domain = self.exit_domain(source, target, kind)?;

        let mut exit = Buffer::new();
        self.collect_exits(configuration, domain, &mut exit)?;

        let mut entry = Buffer::new();
        self.collect_entries(domain, target, last_active, &mut entry)?;

        Ok(self.settle(exit, entry, configuration))
    }

    /// The innermost state the transition stays inside of.
    fn exit_domain(
        &self,
        source: StateId,
        target: StateId,
        kind: TransitionKind,
    ) -> Result<StateId, Error> {
        let enclosing = self.least_common_ancestor(source, target)?;
        match kind {
            TransitionKind::External if enclosing == source => {
                self.parent(source).ok_or(Error::NoCommonAncestor)
            }
            _ => Ok(enclosing),
        }
    }

    /// Every active state the domain contains, innermost first.
    fn collect_exits(
        &self,
        configuration: &Configuration<STATES>,
        domain: StateId,
        exit: &mut Buffer<STATES>,
    ) -> Result<(), Error> {
        for &leaf in configuration {
            if !self.is_ancestor(domain, leaf)? {
                continue;
            }
            let mut cursor = leaf;
            while cursor != domain && !exit.contains(cursor) {
                exit.push(cursor)?;
                cursor = self.parent(cursor).ok_or(Error::NotATree)?;
            }
        }
        self.order_innermost_first(exit)
    }

    fn order_innermost_first(&self, exit: &mut Buffer<STATES>) -> Result<(), Error> {
        let mut depths = [0_usize; STATES];
        let mut index = 0;
        while index < exit.len() {
            depths[index] = self.depth(exit.get(index))?;
            index += 1;
        }

        let mut index = 1;
        while index < exit.len() {
            let mut slot = index;
            while slot > 0 && depths[slot - 1] < depths[slot] {
                depths.swap(slot - 1, slot);
                exit.swap(slot - 1, slot);
                slot -= 1;
            }
            index += 1;
        }
        Ok(())
    }

    /// The named path from below the domain down to the target, outermost
    /// first, then whatever the target contains.
    fn collect_entries(
        &self,
        domain: StateId,
        target: StateId,
        last_active: &LastActive<STATES>,
        entry: &mut Buffer<STATES>,
    ) -> Result<(), Error> {
        let mut path = Buffer::<STATES>::new();
        let mut cursor = target;
        while cursor != domain {
            path.push(cursor)?;
            cursor = self.parent(cursor).ok_or(Error::NoCommonAncestor)?;
        }

        self.descend_along(domain, &path, Restore::Initial, last_active, entry)
    }

    /// Descends from `state`, taking the child on `path` where there is one
    /// and the default otherwise, so a region's subtree stays contiguous and
    /// regions keep their declared order whichever descendant is targeted.
    fn descend_along(
        &self,
        state: StateId,
        path: &Buffer<STATES>,
        inherited: Restore,
        last_active: &LastActive<STATES>,
        entry: &mut Buffer<STATES>,
    ) -> Result<(), Error> {
        let restore = inherited.upgraded_by(self.state(state).history);
        match self.state(state).composition {
            Composition::Atomic => Ok(()),
            Composition::Compound { initial } => {
                let chosen = self.on_path(path, state).map_or_else(
                    || match (restore, last_active.last_child(state)) {
                        (Restore::Subtree, Some(child)) => (child, Restore::Subtree),
                        (Restore::Child, Some(child)) => (child, Restore::Initial),
                        _ => (initial, Restore::Initial),
                    },
                    |child| (child, restore),
                );
                entry.push(chosen.0)?;
                self.descend_along(chosen.0, path, chosen.1, last_active, entry)
            }
            Composition::Parallel => {
                let mut region: StateId = 0;
                let mut entered = false;
                while usize::from(region) < STATES {
                    if self.parent(region) == Some(state) {
                        entered = true;
                        entry.push(region)?;
                        self.descend_along(region, path, restore, last_active, entry)?;
                    }
                    region += 1;
                }
                if entered {
                    Ok(())
                } else {
                    Err(Error::EmptyParallel)
                }
            }
        }
    }

    /// The child of `state` that the explicit path runs through, if any.
    fn on_path(&self, path: &Buffer<STATES>, state: StateId) -> Option<StateId> {
        let mut index = 0;
        while index < path.len() {
            let candidate = path.get(index);
            if self.parent(candidate) == Some(state) {
                return Some(candidate);
            }
            index += 1;
        }
        None
    }

    /// Entering a composite state means entering one of its children, so entry
    /// keeps going until every branch it opened ends at a leaf.
    fn descend(
        &self,
        state: StateId,
        inherited: Restore,
        last_active: &LastActive<STATES>,
        entry: &mut Buffer<STATES>,
    ) -> Result<(), Error> {
        let restore = inherited.upgraded_by(self.state(state).history);
        match self.state(state).composition {
            Composition::Atomic => Ok(()),
            Composition::Compound { initial } => {
                let (child, below) = match (restore, last_active.last_child(state)) {
                    (Restore::Subtree, Some(child)) => (child, Restore::Subtree),
                    (Restore::Child, Some(child)) => (child, Restore::Initial),
                    _ => (initial, Restore::Initial),
                };
                entry.push(child)?;
                self.descend(child, below, last_active, entry)
            }
            Composition::Parallel => {
                let mut region: StateId = 0;
                let mut entered = false;
                while usize::from(region) < STATES {
                    if self.parent(region) == Some(state) {
                        entered = true;
                        entry.push(region)?;
                        self.descend(region, restore, last_active, entry)?;
                    }
                    region += 1;
                }
                if entered {
                    Ok(())
                } else {
                    Err(Error::EmptyParallel)
                }
            }
        }
    }

    /// The leaves left active once the exit set is gone and the entry set is
    /// in.
    fn settle(
        &self,
        exit: Buffer<STATES>,
        entry: Buffer<STATES>,
        previous: &Configuration<STATES>,
    ) -> Transition<STATES> {
        let mut configuration = *previous;
        for &left in exit.as_slice() {
            configuration.remove(left);
        }
        for &joined in entry.as_slice() {
            if self.state(joined).composition == Composition::Atomic {
                configuration.insert(joined);
            }
        }
        Transition::new(exit, entry, configuration)
    }
}

#[cfg(test)]
mod tests;
