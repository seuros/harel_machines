//! Exhaustive reachability over small random charts: the restoring
//! constructors accept exactly what some sequence of honoured calls reaches.
use std::collections::BTreeSet;
use std::vec::Vec;

use super::random::{Rng, slots, states};
use crate::{Chart, Composition, LastActive, StateId, TransitionKind};

const SMALL: usize = 7;
const KINDS: [TransitionKind; 2] = [TransitionKind::External, TransitionKind::Internal];

type Slots = Vec<Option<StateId>>;

/// Every (configuration, history) pair reachable from entering the chart,
/// each transition resolved with or without history and then recorded.
fn reachable(chart: &Chart<SMALL>, n: usize) -> BTreeSet<(Vec<StateId>, Slots)> {
    let ids: Vec<StateId> = (0..).take(n).collect();
    let mut seen = BTreeSet::new();
    let mut frontier = Vec::new();
    for &target in &ids {
        let start = chart.enter(target).unwrap();
        frontier.push((*start.configuration(), LastActive::<SMALL>::new()));
    }
    while let Some((configuration, history)) = frontier.pop() {
        if !seen.insert((configuration.as_slice().to_vec(), slots(&history))) {
            continue;
        }
        for (&source, &target) in ids.iter().flat_map(|s| ids.iter().map(move |t| (s, t))) {
            for kind in KINDS {
                for with in [LastActive::new(), history] {
                    let Ok(step) =
                        chart.resolve_with_history(&configuration, source, target, kind, &with)
                    else {
                        continue;
                    };
                    let mut next = history;
                    next.record(chart, step.exit()).unwrap();
                    frontier.push((*step.configuration(), next));
                }
            }
        }
    }
    seen
}

/// Every subset of `items`.
fn subsets<T: Copy>(items: &[T]) -> impl Iterator<Item = Vec<T>> + '_ {
    (0..1_u32 << items.len()).map(move |mask| {
        (0..items.len())
            .filter(|bit| mask & (1 << bit) != 0)
            .map(|bit| items[bit])
            .collect()
    })
}

/// Every history a chart's slots could hold: each compound state empty or
/// naming one of its children.
fn histories(chart: &Chart<SMALL>) -> Vec<Vec<(StateId, StateId)>> {
    let mut all = std::vec![Vec::new()];
    for (parent, state) in (0..).zip(chart.states()) {
        let Composition::Compound { .. } = state.composition else {
            continue;
        };
        let children: Vec<StateId> = (0..)
            .zip(chart.states())
            .filter(|(_, child)| child.parent == Some(parent))
            .map(|(id, _)| id)
            .collect();
        all = all
            .into_iter()
            .flat_map(|pairs: Vec<(StateId, StateId)>| {
                let mut options = std::vec![pairs.clone()];
                for &child in &children {
                    let mut with = pairs.clone();
                    with.push((parent, child));
                    options.push(with);
                }
                options
            })
            .collect();
    }
    all
}

#[test]
fn restoring_constructors_accept_exactly_the_reachable_states() {
    for seed in 0..300 {
        let mut rng = Rng::new(seed);
        let n = 1 + rng.below(SMALL - 1);
        let chart = Chart::<SMALL>::new(&states(&mut rng, n)).unwrap();
        let seen = reachable(&chart, n);
        let configurations: BTreeSet<Vec<StateId>> = seen.iter().map(|(c, _)| c.clone()).collect();
        let remembered: BTreeSet<Slots> = seen.iter().map(|(_, h)| h.clone()).collect();

        let leaves: Vec<StateId> = (0..)
            .zip(chart.states())
            .filter(|(_, state)| state.composition == Composition::Atomic)
            .map(|(id, _)| id)
            .collect();
        for subset in subsets(&leaves) {
            let accepted = chart.configuration(&subset).is_ok();
            let honest = subset.is_empty() || configurations.contains(&subset);
            assert_eq!(accepted, honest, "seed {seed}: leaves {subset:?}");
        }
        for pairs in histories(&chart) {
            let restored = LastActive::restored(&chart, &pairs);
            let honest = restored.map_or_else(
                |_| {
                    let mut wanted = std::vec![None; SMALL];
                    for &(parent, child) in &pairs {
                        wanted[usize::from(parent)] = Some(child);
                    }
                    remembered.contains(&wanted)
                },
                |history| remembered.contains(&slots(&history)),
            );
            assert_eq!(restored.is_ok(), honest, "seed {seed}: history {pairs:?}");
        }
    }
}

/// Every ordering of `items`.
fn orderings(items: &[StateId]) -> Vec<Vec<StateId>> {
    if items.len() <= 1 {
        return std::vec![items.to_vec()];
    }
    let mut all = Vec::new();
    for (index, &first) in items.iter().enumerate() {
        let mut rest = items.to_vec();
        rest.remove(index);
        for mut tail in orderings(&rest) {
            tail.insert(0, first);
            all.push(tail);
        }
    }
    all
}

#[test]
fn record_accepts_exactly_the_exit_sets_transitions_produce() {
    for seed in 0..120 {
        let mut rng = Rng::new(seed);
        let n = 1 + rng.below(SMALL - 1);
        let chart = Chart::<SMALL>::new(&states(&mut rng, n)).unwrap();
        let ids: Vec<StateId> = (0..).take(n).collect();
        let configurations: BTreeSet<Vec<StateId>> =
            reachable(&chart, n).into_iter().map(|(c, _)| c).collect();
        let mut produced = BTreeSet::new();
        for leaves in &configurations {
            let configuration = chart.configuration(leaves).unwrap();
            for (&source, &target) in ids.iter().flat_map(|s| ids.iter().map(move |t| (s, t))) {
                for kind in KINDS {
                    if let Ok(step) = chart.resolve(&configuration, source, target, kind) {
                        produced.insert(step.exit().to_vec());
                    }
                }
            }
        }
        for subset in subsets(&ids) {
            let candidates = if subset.len() <= 5 {
                orderings(&subset)
            } else {
                let mut reversed = subset.clone();
                reversed.reverse();
                std::vec![subset.clone(), reversed]
            };
            for exited in candidates {
                let accepted = LastActive::<SMALL>::new().record(&chart, &exited).is_ok();
                assert_eq!(
                    accepted,
                    produced.contains(&exited),
                    "seed {seed}: exit {exited:?}"
                );
            }
        }
    }
}
