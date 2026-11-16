//! What a chart refuses to be built from.
use crate::{Chart, ChartError, Composition, History, State};

fn build<const N: usize>(states: &[State]) -> Result<Chart<N>, ChartError> {
    Chart::<N>::new(states)
}

#[test]
fn a_chart_needs_a_state_and_room_for_every_one() {
    assert_eq!(build::<2>(&[]), Err(ChartError::Empty));
    let three = [
        State::parallel(None),
        State::atomic(Some(0)),
        State::atomic(Some(0)),
    ];
    assert_eq!(build::<2>(&three), Err(ChartError::TooMany));
    assert!(build::<3>(&three).is_ok());
}

#[test]
fn a_shorter_slice_leaves_the_rest_of_the_capacity_empty() {
    let chart = build::<8>(&[State::atomic(None)]).unwrap();

    assert_eq!(chart.len(), 1);
    assert!(!chart.is_empty());
    assert_eq!(chart.states(), &[State::atomic(None)]);
    assert_eq!(chart.enter(0).unwrap().configuration().as_slice(), &[0]);
}

#[test]
fn a_parent_or_initial_child_past_the_last_state_is_unknown() {
    assert_eq!(
        build::<4>(&[State::compound(None, 1), State::atomic(Some(7))]),
        Err(ChartError::UnknownState)
    );
    assert_eq!(
        build::<4>(&[State::compound(None, 9), State::atomic(Some(0))]),
        Err(ChartError::UnknownState)
    );
}

#[test]
fn an_atomic_state_cannot_contain_states() {
    let buried = [State::atomic(None), State::atomic(Some(0))];
    assert_eq!(build::<2>(&buried), Err(ChartError::AtomicParent));
    let own = [State::atomic(Some(0))];
    assert_eq!(build::<1>(&own), Err(ChartError::AtomicParent));
}

#[test]
fn an_initial_child_must_be_a_child_of_its_compound_state() {
    let outside = [
        State::compound(None, 2),
        State::compound(Some(0), 2),
        State::atomic(Some(1)),
    ];
    assert_eq!(build::<3>(&outside), Err(ChartError::InitialNotChild));
    let itself = [State::compound(None, 0)];
    assert_eq!(build::<1>(&itself), Err(ChartError::InitialNotChild));
}

#[test]
fn a_parallel_state_needs_a_region() {
    let barren = [State::compound(None, 1), State::parallel(Some(0))];
    assert_eq!(build::<2>(&barren), Err(ChartError::EmptyParallel));
}

#[test]
fn an_atomic_state_has_no_history() {
    let marked = [
        State::compound(None, 1),
        State::atomic(Some(0)).with_history(History::Deep),
    ];
    assert_eq!(build::<2>(&marked), Err(ChartError::AtomicHistory));
}

#[test]
fn a_chart_is_one_tree() {
    let forest = [State::atomic(None), State::atomic(None)];
    assert_eq!(build::<2>(&forest), Err(ChartError::MultipleRoots));
}

#[test]
fn a_parent_cycle_is_not_a_tree() {
    let knot = [
        State::compound(None, 1),
        State::atomic(Some(0)),
        State::compound(Some(3), 3),
        State::compound(Some(2), 2),
    ];
    assert_eq!(build::<4>(&knot), Err(ChartError::NotATree));
    let rootless = [State::compound(Some(1), 1), State::compound(Some(0), 0)];
    assert_eq!(build::<2>(&rootless), Err(ChartError::NotATree));
}

/// Two faults answer the one listed first, whatever order the states are in.
#[test]
fn checks_run_in_the_order_of_the_variants() {
    let both = [
        State::parallel(None),
        State::compound(Some(0), 2),
        State::atomic(Some(0)).with_history(History::Shallow),
        State::atomic(Some(2)),
    ];
    assert_eq!(build::<4>(&both), Err(ChartError::AtomicParent));
    let later = [
        State::parallel(None),
        State::atomic(Some(0)).with_history(History::Shallow),
        State::parallel(Some(0)),
    ];
    assert_eq!(build::<3>(&later), Err(ChartError::EmptyParallel));
}

#[test]
fn every_chart_error_says_what_is_wrong() {
    use std::string::ToString;
    let all = [
        ChartError::Empty,
        ChartError::TooMany,
        ChartError::UnknownState,
        ChartError::AtomicParent,
        ChartError::InitialNotChild,
        ChartError::EmptyParallel,
        ChartError::AtomicHistory,
        ChartError::MultipleRoots,
        ChartError::NotATree,
    ];
    for error in all {
        assert_ne!(error.to_string(), "");
    }
    assert_eq!(Composition::default(), Composition::Atomic);
}
