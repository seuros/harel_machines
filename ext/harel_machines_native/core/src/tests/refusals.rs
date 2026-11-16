//! Every refusal, and that a refused call changes nothing.
use super::charts::{leaves, player, radio, workflow};
use crate::{Configuration, History, LastActive, Refusal, TransitionKind};

const EXTERNAL: TransitionKind = TransitionKind::External;

#[test]
fn an_index_past_the_last_state_is_unknown_to_every_call() {
    let chart = player();
    let start = leaves(&chart, &[1]);
    let unknown = Err(Refusal::UnknownState);

    assert_eq!(chart.state(5).map(drop), unknown);
    assert_eq!(chart.parent(5).map(drop), unknown);
    assert_eq!(chart.depth(5).map(drop), unknown);
    assert_eq!(chart.is_ancestor(0, 5).map(drop), unknown);
    assert_eq!(chart.is_ancestor(5, 0).map(drop), unknown);
    assert_eq!(chart.least_common_ancestor(5, 1).map(drop), unknown);
    assert_eq!(chart.enter(5).map(drop), unknown);
    assert_eq!(chart.resolve(&start, 1, 5, EXTERNAL).map(drop), unknown);
    assert_eq!(chart.resolve(&start, 5, 1, EXTERNAL).map(drop), unknown);
    assert_eq!(chart.configuration(&[1, 5]).map(drop), unknown);
}

#[test]
fn a_source_outside_the_configuration_is_inactive() {
    let chart = player();
    let stopped = leaves(&chart, &[1]);

    assert_eq!(
        chart.resolve(&stopped, 2, 1, EXTERNAL),
        Err(Refusal::Inactive)
    );
    assert_eq!(
        chart.resolve(&stopped, 3, 1, EXTERNAL),
        Err(Refusal::Inactive)
    );
    let nothing = Configuration::new();
    assert_eq!(
        chart.resolve(&nothing, 0, 1, EXTERNAL),
        Err(Refusal::Inactive)
    );
    assert!(chart.resolve(&stopped, 0, 2, EXTERNAL).is_ok());
}

#[test]
fn only_configurations_the_chart_can_be_in_are_accepted() {
    let chart = radio(History::None);
    let invalid = Err(Refusal::InvalidConfiguration);

    assert_eq!(chart.configuration(&[]), Ok(Configuration::new()));
    assert_eq!(chart.configuration(&[7, 4]).map(|c| c.len()), Ok(2));
    // A region left out of a parallel state.
    assert_eq!(chart.configuration(&[4]), invalid);
    // Two children of one compound state.
    assert_eq!(chart.configuration(&[4, 5, 7]), invalid);
    // Two branches of the root.
    assert_eq!(chart.configuration(&[1, 4, 7]), invalid);
    // A state that is not a leaf, and a leaf named twice.
    assert_eq!(chart.configuration(&[3, 7]), invalid);
    assert_eq!(chart.configuration(&[4, 7, 4]), invalid);
}

/// A configuration is a set of indices: one of another chart of the same
/// capacity reaches this one only by its indices, which are checked.
#[test]
fn a_configuration_of_another_chart_is_refused() {
    let chart = radio(History::None);
    let other = crate::Chart::<9>::new(&[
        crate::State::parallel(None),
        crate::State::atomic(Some(0)),
        crate::State::atomic(Some(0)),
    ])
    .unwrap();
    let foreign = other.configuration(&[1, 2]).unwrap();

    assert_eq!(
        chart.resolve(&foreign, 1, 2, EXTERNAL),
        Err(Refusal::InvalidConfiguration)
    );
}

#[test]
fn a_history_of_another_chart_is_refused_and_changes_nothing() {
    let radio = radio(History::Deep);
    let mut foreign = LastActive::<9>::new();
    let on = radio.enter(5).unwrap();
    let off = radio.resolve(on.configuration(), 2, 1, EXTERNAL).unwrap();
    foreign.record(&radio, off.exit()).unwrap();
    let other = crate::Chart::<9>::new(&[
        crate::State::parallel(None),
        crate::State::atomic(Some(0)),
        crate::State::atomic(Some(0)),
    ])
    .unwrap();
    let both = other.configuration(&[1, 2]).unwrap();

    assert_eq!(
        other.resolve_with_history(&both, 1, 2, EXTERNAL, &foreign),
        Err(Refusal::InvalidHistory)
    );
    assert_eq!(
        other.enter_with_history(0, &foreign),
        Err(Refusal::InvalidHistory)
    );

    // An exit set no transition of the pipeline has: Load without Parse.
    let pipeline = super::charts::pipeline();
    let mut mine = LastActive::<6>::new();
    let before = mine;
    assert_eq!(mine.record(&pipeline, &[3]), Err(Refusal::InvalidHistory));
    assert_eq!(mine.record(&pipeline, &[9]), Err(Refusal::UnknownState));
    assert_eq!(mine, before);
}

#[test]
fn a_history_no_transitions_leave_is_refused_where_it_is_read() {
    let chart = workflow(History::Deep);
    let invalid = Err(Refusal::InvalidHistory);

    // Work remembers Fetch, but Fetch, which is exited with it, remembers
    // nothing.
    assert_eq!(LastActive::restored(&chart, &[(2, 3)]).map(drop), invalid);
    // Not a compound state, not its child, named twice.
    assert_eq!(LastActive::restored(&chart, &[(4, 5)]).map(drop), invalid);
    assert_eq!(LastActive::restored(&chart, &[(2, 4)]).map(drop), invalid);
    assert_eq!(
        LastActive::restored(&chart, &[(3, 4), (3, 5)]).map(drop),
        invalid
    );
    assert_eq!(
        LastActive::restored(&chart, &[(3, 9)]).map(drop),
        Err(Refusal::UnknownState)
    );
    let held = LastActive::restored(&chart, &[(3, 5), (2, 3)]).unwrap();
    assert_eq!(held.last_child(2), Some(3));
    assert_eq!(held.last_child(3), Some(5));
}

#[test]
fn a_refused_record_files_nothing() {
    let chart = workflow(History::Deep);
    let mut remembered = LastActive::new();
    let before = remembered;

    // Fetch exited without the leaf it held.
    assert_eq!(
        remembered.record(&chart, &[3]),
        Err(Refusal::InvalidHistory)
    );
    assert_eq!(
        remembered.record(&chart, &[4, 70]),
        Err(Refusal::UnknownState)
    );
    assert_eq!(remembered, before);
    assert_eq!(remembered.record(&chart, &[5, 3]), Ok(()));
    assert_eq!(remembered.last_child(2), Some(3));
}

#[test]
fn every_refusal_says_why() {
    use std::string::ToString;
    let all = [
        Refusal::UnknownState,
        Refusal::InvalidConfiguration,
        Refusal::InvalidHistory,
        Refusal::Inactive,
    ];
    for refusal in all {
        assert_ne!(refusal.to_string(), "");
    }
}
