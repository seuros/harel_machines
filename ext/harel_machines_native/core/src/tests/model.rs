//! The reference the core is compared against: the textbook statechart
//! resolution, written for clarity over a heap (recursion, `Vec`, a stable
//! sort), with none of the core's fixed-capacity machinery. It trusts its
//! input; the random tests only hand it charts that are trees.
use std::vec::Vec;

use crate::{Composition, History, State, StateId, TransitionKind};

/// How far down an entry reads what a state last held.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Restore {
    Initial,
    Child,
    Subtree,
}

/// One resolved transition: exit, entry, and the leaves it settles in.
pub type Step = (Vec<StateId>, Vec<StateId>, Vec<StateId>);

pub struct Model {
    pub states: Vec<State>,
}

impl Model {
    fn state(&self, id: StateId) -> State {
        self.states[usize::from(id)]
    }

    fn parent(&self, id: StateId) -> Option<StateId> {
        self.state(id).parent
    }

    pub fn root(&self) -> StateId {
        self.ids().find(|&id| self.parent(id).is_none()).unwrap()
    }

    pub fn ids(&self) -> impl Iterator<Item = StateId> + use<> {
        (0..u16::try_from(self.states.len()).unwrap())
            .collect::<Vec<_>>()
            .into_iter()
    }

    pub fn depth(&self, id: StateId) -> usize {
        self.parent(id).map_or(0, |parent| self.depth(parent) + 1)
    }

    pub fn is_ancestor(&self, ancestor: StateId, descendant: StateId) -> bool {
        self.parent(descendant)
            .is_some_and(|parent| parent == ancestor || self.is_ancestor(ancestor, parent))
    }

    pub fn least_common_ancestor(&self, left: StateId, right: StateId) -> StateId {
        if left == right || self.is_ancestor(left, right) {
            left
        } else {
            self.least_common_ancestor(self.parent(left).unwrap(), right)
        }
    }

    pub fn children(&self, id: StateId) -> Vec<StateId> {
        self.ids()
            .filter(|&child| self.parent(child) == Some(id))
            .collect()
    }

    pub fn atomic(&self, id: StateId) -> bool {
        self.state(id).composition == Composition::Atomic
    }

    /// The leaves and every state containing one.
    pub fn active(&self, leaves: &[StateId]) -> Vec<StateId> {
        self.ids()
            .filter(|&id| {
                leaves
                    .iter()
                    .any(|&leaf| leaf == id || self.is_ancestor(id, leaf))
            })
            .collect()
    }

    fn descend(
        &self,
        state: StateId,
        toward: &[StateId],
        inherited: Restore,
        slots: &[Option<StateId>],
        entry: &mut Vec<StateId>,
    ) {
        let on_path = toward
            .iter()
            .copied()
            .find(|&id| self.parent(id) == Some(state));
        // History applies from the target down: a state the explicit path
        // runs through opens every child from scratch.
        let restore = match (on_path, inherited, self.state(state).history) {
            (None, Restore::Subtree, _) | (None, _, History::Deep) => Restore::Subtree,
            (None, Restore::Child, _) | (None, _, History::Shallow) => Restore::Child,
            (Some(_), _, _) | (None, Restore::Initial, History::None) => Restore::Initial,
        };
        match self.state(state).composition {
            Composition::Atomic => {}
            Composition::Compound { initial } => {
                let (child, next) = match (on_path, restore, slots[usize::from(state)]) {
                    (Some(child), _, _) => (child, restore),
                    (None, Restore::Subtree, Some(child)) => (child, Restore::Subtree),
                    (None, Restore::Child, Some(child)) => (child, Restore::Initial),
                    _ => (initial, Restore::Initial),
                };
                entry.push(child);
                self.descend(child, toward, next, slots, entry);
            }
            Composition::Parallel => {
                for region in self.children(state) {
                    entry.push(region);
                    self.descend(region, toward, restore, slots, entry);
                }
            }
        }
    }

    /// The states from `target` up to, not including, `boundary`.
    fn path(&self, boundary: StateId, target: StateId) -> Vec<StateId> {
        let mut path = Vec::new();
        let mut cursor = target;
        while cursor != boundary {
            path.push(cursor);
            cursor = self.parent(cursor).unwrap();
        }
        path
    }

    fn settle(&self, previous: &[StateId], exit: Vec<StateId>, entry: Vec<StateId>) -> Step {
        let mut leaves: Vec<StateId> = previous
            .iter()
            .copied()
            .filter(|leaf| !exit.contains(leaf))
            .collect();
        leaves.extend(entry.iter().copied().filter(|&id| self.atomic(id)));
        leaves.sort_unstable();
        leaves.dedup();
        (exit, entry, leaves)
    }

    pub fn enter(&self, target: StateId, slots: &[Option<StateId>]) -> Step {
        let root = self.root();
        let mut entry = Vec::from([root]);
        self.descend(
            root,
            &self.path(root, target),
            Restore::Initial,
            slots,
            &mut entry,
        );
        self.settle(&[], Vec::new(), entry)
    }

    /// `None` when the source is not active.
    pub fn resolve(
        &self,
        leaves: &[StateId],
        source: StateId,
        target: StateId,
        kind: TransitionKind,
        slots: &[Option<StateId>],
    ) -> Option<Step> {
        if !self.active(leaves).contains(&source) {
            return None;
        }
        let boundary = self.least_common_ancestor(source, target);
        let reentered = kind == TransitionKind::External && boundary == source;
        let mut exit = Vec::new();
        for &leaf in leaves {
            if !self.is_ancestor(boundary, leaf) {
                continue;
            }
            let mut cursor = leaf;
            while cursor != boundary && !exit.contains(&cursor) {
                exit.push(cursor);
                cursor = self.parent(cursor).unwrap();
            }
        }
        if reentered {
            exit.push(boundary);
        }
        exit.sort_by_key(|&id| core::cmp::Reverse(self.depth(id)));
        let mut entry = Vec::new();
        if reentered {
            entry.push(boundary);
        }
        let path = self.path(boundary, target);
        self.descend(boundary, &path, Restore::Initial, slots, &mut entry);
        Some(self.settle(leaves, exit, entry))
    }

    /// Files every exited state under a compound parent.
    pub fn record(&self, slots: &mut [Option<StateId>], exit: &[StateId]) {
        for &state in exit {
            if let Some(parent) = self.parent(state)
                && matches!(self.state(parent).composition, Composition::Compound { .. })
            {
                slots[usize::from(parent)] = Some(state);
            }
        }
    }
}
