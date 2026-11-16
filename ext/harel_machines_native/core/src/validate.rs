//! The checks a chart, a configuration and a history pass before anything
//! is resolved over them.

use crate::buffer::{Buffer, broken};
use crate::indexed;
use crate::{
    Chart, ChartError, Composition, Configuration, History, LastActive, Refusal, State, StateId,
};

/// A depth not worked out yet. No depth reaches it: a chart holds at most
/// `u16::MAX` states, so the deepest is at `u16::MAX - 1`.
const UNKNOWN: StateId = StateId::MAX;

/// Checks `states` as one tree and fills each state's depth and child count.
/// Answers the root.
pub fn tree(
    states: &[State],
    depths: &mut [StateId],
    children: &mut [StateId],
) -> Result<StateId, ChartError> {
    let known = |id: StateId| usize::from(id) < states.len();
    for state in states {
        let initial = match state.composition {
            Composition::Compound { initial } => Some(initial),
            Composition::Atomic | Composition::Parallel => None,
        };
        if !state.parent.is_none_or(known) || !initial.is_none_or(known) {
            return Err(ChartError::UnknownState);
        }
    }
    let mut roots = 0_usize;
    let mut root = 0;
    for (id, state) in indexed(states) {
        match state.parent {
            None => (roots, root) = (roots + 1, id),
            Some(parent) => {
                let count = children
                    .get_mut(usize::from(parent))
                    .ok_or(ChartError::UnknownState)?;
                *count = count.checked_add(1).ok_or(ChartError::TooMany)?;
            }
        }
    }
    compositions(states, children)?;
    match roots {
        0 => return Err(ChartError::NotATree),
        1 => {}
        _ => return Err(ChartError::MultipleRoots),
    }
    fill_depths(states, depths)?;
    Ok(root)
}

/// One composition check: whether the state at `id` breaks it, given every
/// state and each one's child count.
type Check = fn(&[State], &[StateId], StateId, &State) -> bool;

/// The composition checks, in [`ChartError`] order: no atomic parent, an
/// initial child of its own, a region in every parallel state, no history on
/// an atomic state. A check is reported before any later one, whichever
/// state breaks it.
const COMPOSITIONS: [(ChartError, Check); 4] = [
    (ChartError::AtomicParent, |_, children, id, state| {
        state.composition == Composition::Atomic && count(children, id) > 0
    }),
    (ChartError::InitialNotChild, |states, _, id, state| {
        let Composition::Compound { initial } = state.composition else {
            return false;
        };
        states
            .get(usize::from(initial))
            .is_none_or(|child| child.parent != Some(id))
    }),
    (ChartError::EmptyParallel, |_, children, id, state| {
        state.composition == Composition::Parallel && count(children, id) == 0
    }),
    (ChartError::AtomicHistory, |_, _, _, state| {
        state.composition == Composition::Atomic && state.history != History::None
    }),
];

fn count(children: &[StateId], id: StateId) -> StateId {
    children.get(usize::from(id)).copied().unwrap_or(0)
}

fn compositions(states: &[State], children: &[StateId]) -> Result<(), ChartError> {
    for (error, broken) in COMPOSITIONS {
        if indexed(states).any(|(id, state)| broken(states, children, id, state)) {
            return Err(error);
        }
    }
    Ok(())
}

/// Every state's depth, each worked out once: a walk up stops at the first
/// state whose depth is known, and a walk longer than the chart is a cycle.
fn fill_depths(states: &[State], depths: &mut [StateId]) -> Result<(), ChartError> {
    let len = states.len();
    for depth in depths.iter_mut().take(len) {
        *depth = UNKNOWN;
    }
    let parent = |id: StateId| states.get(usize::from(id)).and_then(|state| state.parent);
    let depth_of = |depths: &[StateId], id: StateId| depths.get(usize::from(id)).copied();
    for (start, _) in indexed(states) {
        let mut steps = 0_usize;
        let mut cursor = start;
        let base = loop {
            match depth_of(depths, cursor) {
                Some(UNKNOWN) => {}
                Some(known) => break usize::from(known),
                None => return Err(ChartError::UnknownState),
            }
            match parent(cursor) {
                None => break 0,
                Some(above) => {
                    steps += 1;
                    if steps > len {
                        return Err(ChartError::NotATree);
                    }
                    cursor = above;
                }
            }
        };
        let mut depth = base + steps;
        let mut cursor = start;
        loop {
            let slot = depths
                .get_mut(usize::from(cursor))
                .ok_or(ChartError::UnknownState)?;
            if *slot != UNKNOWN {
                break;
            }
            *slot = StateId::try_from(depth).map_err(|_| ChartError::TooMany)?;
            match parent(cursor) {
                Some(above) if depth > 0 => (cursor, depth) = (above, depth - 1),
                _ => break,
            }
        }
    }
    Ok(())
}

/// The states `leaves` makes active, marked by index, once `leaves` is
/// checked as a configuration of `chart`: what [`Chart::configuration`]
/// builds from, and what resolving reads the source's activity from.
pub fn active<const N: usize>(
    chart: &Chart<N>,
    leaves: &[StateId],
) -> Result<([bool; N], [bool; N]), Refusal> {
    let mut leaf = [false; N];
    for &id in leaves {
        let Composition::Atomic = chart.state(id)?.composition else {
            return Err(Refusal::InvalidConfiguration);
        };
        let mark = leaf.get_mut(usize::from(id)).ok_or(Refusal::UnknownState)?;
        if *mark {
            return Err(Refusal::InvalidConfiguration);
        }
        *mark = true;
    }
    let mut active = [false; N];
    let mut held = [0 as StateId; N];
    for &id in leaves {
        let mut cursor = id;
        loop {
            let mark = active
                .get_mut(usize::from(cursor))
                .ok_or(Refusal::UnknownState)?;
            if *mark {
                break;
            }
            *mark = true;
            let Some(parent) = chart.parent(cursor)? else {
                break;
            };
            let count = held
                .get_mut(usize::from(parent))
                .ok_or(Refusal::UnknownState)?;
            *count += 1;
            cursor = parent;
        }
    }
    for ((id, state), (&on, &count)) in indexed(chart.states()).zip(active.iter().zip(&held)) {
        let fits = match state.composition {
            Composition::Atomic => true,
            Composition::Compound { .. } => count == 1,
            Composition::Parallel => count == chart.children(id),
        };
        if on && !fits {
            return Err(Refusal::InvalidConfiguration);
        }
    }
    Ok((leaf, active))
}

/// [`Chart::configuration`].
pub fn leaves<const N: usize>(
    chart: &Chart<N>,
    leaves: &[StateId],
) -> Result<Configuration<N>, Refusal> {
    let (leaf, _) = active(chart, leaves)?;
    Ok(Configuration::from_marks(&leaf))
}

/// Checks `exited` is an exit set some transition over `chart` produces, in
/// the order it produces it.
///
/// A transition exits everything active below its boundary, and the boundary
/// too when it leaves it. So the set is empty, or it is the whole active
/// subtree under one state (each compound state in it has exactly one child
/// in it, each parallel state all of its regions), or it is the whole active
/// subtree of every region of one parallel state. Its order is the one a
/// transition gives it: deepest first, states at one depth in the order of
/// the leaf that put them there, leaves ascending.
pub fn exits<const N: usize>(chart: &Chart<N>, exited: &[StateId]) -> Result<(), Refusal> {
    let mut member = [false; N];
    for &id in exited {
        chart.state(id)?;
        let mark = slot(&mut member, id)?;
        if *mark {
            return Err(Refusal::InvalidHistory);
        }
        *mark = true;
    }
    let inside = |member: &[bool; N], id: StateId| member.get(usize::from(id)) == Some(&true);
    let mut held = [0 as StateId; N];
    let mut tops = 0_usize;
    let mut top_parent = None;
    for &id in exited {
        match chart.parent(id)? {
            Some(parent) if inside(&member, parent) => *slot(&mut held, parent)? += 1,
            parent => {
                tops += 1;
                if tops > 1 && top_parent != parent {
                    return Err(Refusal::InvalidHistory);
                }
                top_parent = parent;
            }
        }
    }
    if tops > 1 {
        let Some(parallel) = top_parent else {
            return Err(Refusal::InvalidHistory);
        };
        let spans = chart.state(parallel)?.composition == Composition::Parallel
            && usize::from(chart.children(parallel)) == tops;
        if !spans {
            return Err(Refusal::InvalidHistory);
        }
    }
    for &id in exited {
        let count = held.get(usize::from(id)).copied().unwrap_or(0);
        let whole = match chart.state(id)?.composition {
            Composition::Atomic => true,
            Composition::Compound { .. } => count == 1,
            Composition::Parallel => count == chart.children(id),
        };
        if !whole {
            return Err(Refusal::InvalidHistory);
        }
    }
    let mut canonical = Buffer::<N>::new();
    let mut taken = [false; N];
    for (id, _) in indexed(chart.states()) {
        let leaf = chart.state(id)?.composition == Composition::Atomic;
        if !leaf || !inside(&member, id) {
            continue;
        }
        let mut cursor = Some(id);
        while let Some(state) = cursor.filter(|&state| inside(&member, state)) {
            let mark = slot(&mut taken, state)?;
            if *mark {
                break;
            }
            *mark = true;
            canonical.push(state)?;
            cursor = chart.parent(state)?;
        }
    }
    chart.order_innermost_first(&mut canonical)?;
    if canonical.as_slice() != exited {
        return Err(Refusal::InvalidHistory);
    }
    Ok(())
}

/// Checks `last_active` is a history some transitions over `chart` leave.
///
/// Every slot belongs to a compound state of the chart and names one of its
/// children. A state is exited with everything active below it, so that
/// child is filled: an atomic state is, a compound state is when its own slot
/// names a filled child, a parallel state is when every region is. And a
/// compound state that remembers nothing was never left nor had a child
/// leave, so at most one of its children has anything remembered below it:
/// the one it was entered at.
pub fn history<const N: usize>(
    chart: &Chart<N>,
    last_active: &LastActive<N>,
) -> Result<(), Refusal> {
    for (parent, slot) in indexed(last_active.slots()) {
        let Some(child) = *slot else { continue };
        let compound = chart
            .state(parent)
            .is_ok_and(|state| matches!(state.composition, Composition::Compound { .. }));
        if !compound || chart.parent(child) != Ok(Some(parent)) {
            return Err(Refusal::InvalidHistory);
        }
    }
    let mut filled = [false; N];
    let mut short = [false; N];
    let mut visited = [0 as StateId; N];
    let order = deepest_first::<N>(chart)?;
    for id in order.iter().take(chart.len()).filter_map(|&key| low(key)) {
        let state = chart.state(id)?;
        let remembered = last_active.last_child(id);
        let below = *slot(&mut visited, id)?;
        let done = match state.composition {
            Composition::Atomic => true,
            Composition::Compound { .. } if remembered.is_none() && below > 1 => {
                return Err(Refusal::InvalidHistory);
            }
            Composition::Compound { .. } => {
                remembered.is_some_and(|child| filled.get(usize::from(child)) == Some(&true))
            }
            Composition::Parallel => !*slot(&mut short, id)?,
        };
        if remembered.is_some_and(|child| filled.get(usize::from(child)) != Some(&true)) {
            return Err(Refusal::InvalidHistory);
        }
        *slot(&mut filled, id)? = done;
        let Some(parent) = state.parent else { continue };
        if remembered.is_some() || below > 0 {
            *slot(&mut visited, parent)? += 1;
        }
        if !done && chart.state(parent)?.composition == Composition::Parallel {
            *slot(&mut short, parent)? = true;
        }
    }
    Ok(())
}

/// The low sixteen bits of a sort key: the state it orders.
fn low(key: u32) -> Option<StateId> {
    StateId::try_from(key & 0xFFFF).ok()
}

/// Every state as a sort key, deepest first, so each child comes before its
/// parent: the inverted depth above the index.
fn deepest_first<const N: usize>(chart: &Chart<N>) -> Result<[u32; N], Refusal> {
    let mut order = [u32::MAX; N];
    for (key, (id, _)) in order.iter_mut().zip(indexed(chart.states())) {
        let depth = StateId::try_from(chart.depth(id)?).map_err(|_| broken("a deep state"))?;
        *key = (u32::from(StateId::MAX - depth) << 16) | u32::from(id);
    }
    order.sort_unstable();
    Ok(order)
}

/// The index of `id` in a scratch array, which a checked index always has.
pub fn slot<T>(array: &mut [T], id: StateId) -> Result<&mut T, Refusal> {
    array
        .get_mut(usize::from(id))
        .ok_or_else(|| broken("a checked index is past the scratch array"))
}
