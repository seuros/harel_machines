//! The core against the reference model over random charts and random walks,
//! and every state a walk reaches rebuilt through the restoring
//! constructors.
use std::vec::Vec;

use super::model::Model;
use crate::{Chart, Composition, History, LastActive, Refusal, State, StateId, TransitionKind};

pub const CAPACITY: usize = 12;

/// A small xorshift generator: deterministic, so a failure replays.
pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub const fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// A value in `0..bound`.
    pub fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).unwrap()).unwrap()
    }

    pub fn id(&mut self, bound: usize) -> StateId {
        StateId::try_from(self.below(bound)).unwrap()
    }
}

/// A random tree of `n` states: each state's parent is drawn among the ones
/// before it, every state with children is compound or parallel, any of them
/// may carry history, and the indices are shuffled so the root and the
/// regions sit anywhere.
pub fn states(rng: &mut Rng, n: usize) -> Vec<State> {
    let parents: Vec<Option<usize>> = (0..n).map(|id| (id > 0).then(|| rng.below(id))).collect();
    let mut order: Vec<usize> = (0..n).collect();
    for index in (1..n).rev() {
        order.swap(index, rng.below(index + 1));
    }
    let id = |old: usize| StateId::try_from(order[old]).unwrap();
    let mut states = std::vec![State::atomic(None); n];
    for old in 0..n {
        let children: Vec<usize> = (0..n).filter(|&c| parents[c] == Some(old)).collect();
        let history = [History::None, History::Shallow, History::Deep][rng.below(3)];
        let state = if children.is_empty() {
            State::atomic(None)
        } else if rng.below(4) == 0 {
            State::parallel(None).with_history(history)
        } else {
            let initial = children[rng.below(children.len())];
            State::compound(None, id(initial)).with_history(history)
        };
        states[order[old]] = State {
            parent: parents[old].map(id),
            ..state
        };
    }
    states
}

/// Every slot of `history`, by index.
pub fn slots<const N: usize>(history: &LastActive<N>) -> Vec<Option<StateId>> {
    (0..N)
        .map(|id| history.last_child(StateId::try_from(id).unwrap()))
        .collect()
}

fn kind(rng: &mut Rng) -> TransitionKind {
    [TransitionKind::External, TransitionKind::Internal][rng.below(2)]
}

/// Queries agree with the model on every pair.
fn queries(chart: &Chart<CAPACITY>, model: &Model) -> Result<(), Refusal> {
    assert_eq!(chart.root(), model.root());
    for left in model.ids() {
        assert_eq!(chart.depth(left)?, model.depth(left));
        for right in model.ids() {
            assert_eq!(
                chart.is_ancestor(left, right)?,
                model.is_ancestor(left, right)
            );
            let lca = chart.least_common_ancestor(left, right)?;
            assert_eq!(lca, model.least_common_ancestor(left, right));
        }
    }
    Ok(())
}

#[test]
fn the_core_resolves_as_the_reference_model_does() -> Result<(), Refusal> {
    for seed in 0..400 {
        let mut rng = Rng::new(seed);
        let n = 1 + rng.below(CAPACITY);
        let model = Model {
            states: states(&mut rng, n),
        };
        let chart = Chart::<CAPACITY>::new(&model.states).unwrap();
        queries(&chart, &model)?;
        let mut history = LastActive::new();
        let mut reference = std::vec![None; CAPACITY];
        let target = rng.id(n);
        let start = chart.enter_with_history(target, &history)?;
        let (exit, entry, leaves) = model.enter(target, &reference);
        assert_eq!(
            (start.exit(), start.entry()),
            (&exit[..], &entry[..]),
            "seed {seed}"
        );
        assert_eq!(start.configuration().as_slice(), &leaves[..]);
        let mut configuration = *start.configuration();
        for _ in 0..40 {
            let (source, target, kind) = (rng.id(n), rng.id(n), kind(&mut rng));
            let remembered = rng.below(2) == 0;
            let (with, held) = if remembered {
                (history, reference.clone())
            } else {
                (LastActive::new(), std::vec![None; CAPACITY])
            };
            let leaves = configuration.as_slice().to_vec();
            let core = chart.resolve_with_history(&configuration, source, target, kind, &with);
            let Some((exit, entry, after)) = model.resolve(&leaves, source, target, kind, &held)
            else {
                assert_eq!(core, Err(Refusal::Inactive), "seed {seed}");
                continue;
            };
            let step = core?;
            assert_eq!(step.exit(), &exit[..], "seed {seed} {source}->{target}");
            assert_eq!(step.entry(), &entry[..], "seed {seed} {source}->{target}");
            assert_eq!(step.configuration().as_slice(), &after[..]);
            history.record(&chart, step.exit())?;
            model.record(&mut reference, &exit);
            assert_eq!(slots(&history), reference);
            configuration = *step.configuration();
            restores(&chart, &configuration, &history)?;
        }
    }
    Ok(())
}

/// What a walk reached rebuilds, through the restoring constructors, into
/// exactly itself: they never refuse an honest state.
fn restores(
    chart: &Chart<CAPACITY>,
    configuration: &crate::Configuration<CAPACITY>,
    history: &LastActive<CAPACITY>,
) -> Result<(), Refusal> {
    let mut leaves = configuration.as_slice().to_vec();
    leaves.reverse();
    assert_eq!(chart.configuration(&leaves)?, *configuration);
    let saved: Vec<_> = history.remembered().collect();
    assert_eq!(LastActive::restored(chart, &saved)?, *history);
    Ok(())
}

/// A chart rebuilt from its own states is the same chart, and one with a
/// random fault is refused.
#[test]
fn a_chart_rebuilds_from_its_states_and_refuses_a_broken_one() {
    for seed in 0..400 {
        let mut rng = Rng::new(seed);
        let n = 1 + rng.below(CAPACITY);
        let states = states(&mut rng, n);
        let chart = Chart::<CAPACITY>::new(&states).unwrap();
        assert_eq!(Chart::<CAPACITY>::new(chart.states()), Ok(chart));
        let mut broken = states.clone();
        let victim = rng.below(n);
        match broken[victim].composition {
            // Another root: refused, as MultipleRoots or as whatever its old
            // parent now lacks.
            Composition::Atomic if broken[victim].parent.is_some() => {
                broken[victim].parent = None;
                assert!(Chart::<CAPACITY>::new(&broken).is_err(), "seed {seed}");
            }
            Composition::Atomic => {}
            Composition::Compound { .. } | Composition::Parallel => {
                broken[victim].composition = Composition::Atomic;
                broken[victim].history = History::None;
                let refused = Err(crate::ChartError::AtomicParent);
                assert_eq!(Chart::<CAPACITY>::new(&broken), refused, "seed {seed}");
            }
        }
    }
}
