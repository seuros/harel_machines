//! Resolving a transition over the tree: the exit set, the entry set and the
//! configuration they leave.

use crate::buffer::{Buffer, broken};
use crate::validate::{self, slot};
use crate::{
    Chart, Composition, Configuration, History, LastActive, Refusal, StateId, Transition,
    TransitionKind,
};

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

/// A state waiting to be descended into, and whether it joins the entry set
/// first.
#[derive(Debug, Clone, Copy)]
struct Pending {
    state: StateId,
    restore: Restore,
    joins: bool,
}

/// The states still to descend into, at most one entry per state.
struct Stack<const N: usize> {
    items: [Pending; N],
    len: usize,
}

impl<const N: usize> Stack<N> {
    const fn new() -> Self {
        let idle = Pending {
            state: 0,
            restore: Restore::Initial,
            joins: false,
        };
        Self {
            items: [idle; N],
            len: 0,
        }
    }

    fn push(&mut self, pending: Pending) -> Result<(), Refusal> {
        let place = self
            .items
            .get_mut(self.len)
            .ok_or_else(|| broken("a state was queued for entry twice"))?;
        *place = pending;
        self.len += 1;
        Ok(())
    }

    fn pop(&mut self) -> Option<Pending> {
        self.len = self.len.checked_sub(1)?;
        self.items.get(self.len).copied()
    }
}

impl<const STATES: usize> Chart<STATES> {
    /// The entry set and configuration for entering `target` from nothing:
    /// every state from the root down to `target`, outermost first, then
    /// whatever `target` contains, down to a leaf in every region opened.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownState`] for an index past the last state.
    pub fn enter(&self, target: StateId) -> Result<Transition<STATES>, Refusal> {
        self.enter_with_history(target, &LastActive::new())
    }

    /// As [`Chart::enter`], with history states restored from `last_active`.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownState`] for an index past the last state,
    /// [`Refusal::InvalidHistory`] if `last_active` holds a child this chart
    /// cannot have remembered.
    pub fn enter_with_history(
        &self,
        target: StateId,
        last_active: &LastActive<STATES>,
    ) -> Result<Transition<STATES>, Refusal> {
        self.state(target)?;
        validate::history(self, last_active)?;
        let mut entry = Buffer::new();
        entry.push(self.root())?;
        self.descend(self.root(), target, last_active, &mut entry)?;
        self.settle(Buffer::new(), entry, &Configuration::new())
    }

    /// Which states a transition from `source` to `target` must exit and
    /// enter.
    ///
    /// What gets exited is read from `configuration`, which is what makes a
    /// parallel region's other leaves come along. Composite states on the way
    /// down are entered at their initial child; see
    /// [`Chart::resolve_with_history`] to restore what they last held.
    ///
    /// The least common ancestor of source and target bounds both sets:
    /// nothing above it is touched, and a region it does not contain is left
    /// as it is. An external transition whose source is that ancestor (a
    /// self-transition, or a transition into the source's own subtree) exits
    /// and re-enters the source itself, and no more than that, so its sibling
    /// regions keep the leaves they held. The root is a source like any other.
    /// When the source contains the target, an internal transition keeps the
    /// source itself active; its active descendants may exit and re-enter.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownState`] for an index past the last state,
    /// [`Refusal::InvalidConfiguration`] for a configuration of another chart,
    /// [`Refusal::Inactive`] when `source` is not active in `configuration`,
    /// checked in that order.
    pub fn resolve(
        &self,
        configuration: &Configuration<STATES>,
        source: StateId,
        target: StateId,
        kind: TransitionKind,
    ) -> Result<Transition<STATES>, Refusal> {
        self.resolve_with_history(configuration, source, target, kind, &LastActive::new())
    }

    /// As [`Chart::resolve`], restoring history states from `last_active`.
    ///
    /// History only applies where entry has a child to choose: at the target
    /// and below it. The states named between the least common ancestor and
    /// the target are entered as named.
    ///
    /// # Errors
    ///
    /// As [`Chart::resolve`], and [`Refusal::InvalidHistory`] (checked after
    /// the configuration) if `last_active` holds a child this chart cannot
    /// have remembered.
    pub fn resolve_with_history(
        &self,
        configuration: &Configuration<STATES>,
        source: StateId,
        target: StateId,
        kind: TransitionKind,
        last_active: &LastActive<STATES>,
    ) -> Result<Transition<STATES>, Refusal> {
        self.state(source)?;
        self.state(target)?;
        let (_, active) = validate::active(self, configuration.as_slice())?;
        validate::history(self, last_active)?;
        if !active.get(usize::from(source)).copied().unwrap_or(false) {
            return Err(Refusal::Inactive);
        }
        let boundary = self.least_common_ancestor(source, target)?;
        let reentered = kind == TransitionKind::External && boundary == source;

        let exit = self.exits(configuration, boundary, reentered)?;
        let mut entry = Buffer::new();
        if reentered {
            entry.push(boundary)?;
        }
        self.descend(boundary, target, last_active, &mut entry)?;
        self.settle(exit, entry, configuration)
    }

    /// Every active state below `boundary`, innermost first, and the boundary
    /// last when the transition leaves it too. States at the same depth leave
    /// in the order of the active leaf that put them here.
    fn exits(
        &self,
        configuration: &Configuration<STATES>,
        boundary: StateId,
        reentered: bool,
    ) -> Result<Buffer<STATES>, Refusal> {
        let mut exit = Buffer::new();
        let mut taken = [false; STATES];
        for &leaf in configuration {
            if !self.is_ancestor(boundary, leaf)? {
                continue;
            }
            let mut cursor = leaf;
            while cursor != boundary && !*slot(&mut taken, cursor)? {
                *slot(&mut taken, cursor)? = true;
                exit.push(cursor)?;
                cursor = self.up(cursor)?;
            }
        }
        if reentered {
            exit.push(boundary)?;
        }
        self.order_innermost_first(&mut exit)?;
        Ok(exit)
    }

    /// Sorts by depth, deepest first, keeping the order of equal depths: each
    /// key is the inverted depth above the position.
    pub(crate) fn order_innermost_first(&self, exit: &mut Buffer<STATES>) -> Result<(), Refusal> {
        let original = *exit;
        let mut keys = [0_u32; STATES];
        for ((key, &state), position) in keys.iter_mut().zip(original.as_slice()).zip(0_u32..) {
            let depth = StateId::try_from(self.depth(state)?)
                .map_err(|_| broken("a depth past u16::MAX"))?;
            *key = (u32::from(StateId::MAX - depth) << 16) | position;
        }
        let keys = keys.get_mut(..original.as_slice().len()).unwrap_or(&mut []);
        keys.sort_unstable();
        for (out, key) in exit.as_mut_slice().iter_mut().zip(keys.iter()) {
            let position = usize::try_from(key & 0xFFFF).unwrap_or(usize::MAX);
            *out = *original
                .as_slice()
                .get(position)
                .ok_or_else(|| broken("a sort key past the exit set"))?;
        }
        Ok(())
    }

    /// Descends from `start` (already handled by the caller), taking the
    /// child toward `target` where there is one and the default otherwise,
    /// so a region's subtree stays contiguous and regions keep their index
    /// order whichever descendant is targeted. Entry keeps going until every
    /// branch it opened ends at a leaf.
    ///
    /// The walk is iterative over a stack of at most one entry per state, so
    /// a deep chart cannot exhaust the call stack.
    fn descend(
        &self,
        start: StateId,
        target: StateId,
        last_active: &LastActive<STATES>,
        entry: &mut Buffer<STATES>,
    ) -> Result<(), Refusal> {
        let mut toward = [None::<StateId>; STATES];
        let mut cursor = target;
        while cursor != start {
            let parent = self.up(cursor)?;
            *slot(&mut toward, parent)? = Some(cursor);
            cursor = parent;
        }
        let mut stack = Stack::<STATES>::new();
        stack.push(Pending {
            state: start,
            restore: Restore::Initial,
            joins: false,
        })?;
        while let Some(Pending {
            state,
            restore,
            joins,
        }) = stack.pop()
        {
            if joins {
                entry.push(state)?;
            }
            let held = self.state(state)?;
            let toward = toward.get(usize::from(state)).copied().flatten();
            // A state above the target is entered (or stays) as named: its
            // own history and any enclosing one restore nothing on the way
            // down, so every child it opens starts from scratch. History
            // applies from the target down.
            let restore = if toward.is_some() {
                Restore::Initial
            } else {
                restore.upgraded_by(held.history)
            };
            match held.composition {
                Composition::Atomic => {}
                Composition::Compound { initial } => {
                    let (state, restore) = toward.map_or_else(
                        || match (restore, last_active.last_child(state)) {
                            (Restore::Subtree, Some(child)) => (child, Restore::Subtree),
                            (Restore::Child, Some(child)) => (child, Restore::Initial),
                            _ => (initial, Restore::Initial),
                        },
                        |child| (child, restore),
                    );
                    stack.push(Pending {
                        state,
                        restore,
                        joins: true,
                    })?;
                }
                Composition::Parallel => {
                    for region in (0..self.len())
                        .rev()
                        .filter_map(|id| StateId::try_from(id).ok())
                    {
                        if self.parent(region)? == Some(state) {
                            stack.push(Pending {
                                state: region,
                                restore,
                                joins: true,
                            })?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// The transition: the leaves left active once the exit set is gone and
    /// the entry set is in.
    fn settle(
        &self,
        exit: Buffer<STATES>,
        entry: Buffer<STATES>,
        previous: &Configuration<STATES>,
    ) -> Result<Transition<STATES>, Refusal> {
        let mut leaf = [false; STATES];
        for &id in previous {
            *slot(&mut leaf, id)? = true;
        }
        for &id in exit.as_slice() {
            *slot(&mut leaf, id)? = false;
        }
        for &id in entry.as_slice() {
            if self.state(id)?.composition == Composition::Atomic {
                *slot(&mut leaf, id)? = true;
            }
        }
        let configuration = Configuration::from_marks(&leaf);
        Ok(Transition::new(exit, entry, configuration))
    }
}
