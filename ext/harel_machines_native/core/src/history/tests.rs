use crate::{Chart, Configuration, Error, History, LastActive, State, TransitionKind};

/// ```text
/// 0 Root
/// |- 1 Idle
/// `- 2 Work
///    |- 3 Fetch
///    `- 4 Done
/// ```
const WORKFLOW: Chart<5> = Chart::new([
    State::compound(None, 1),
    State::atomic(Some(0)),
    State::compound(Some(0), 3).with_history(History::Shallow),
    State::atomic(Some(2)),
    State::atomic(Some(2)),
]);

#[test]
fn recording_files_every_exited_state_under_the_state_that_held_it() -> Result<(), Error> {
    let mut configuration = Configuration::<5>::new();
    configuration.insert(4);
    let step = WORKFLOW.resolve(&configuration, 2, 1, TransitionKind::External)?;

    let mut remembered = LastActive::new();
    remembered.record(&WORKFLOW, &step);

    assert_eq!(remembered.last_child(2), Some(4));
    assert_eq!(remembered.last_child(0), Some(2));
    assert_eq!(remembered.last_child(4), None);
    Ok(())
}

#[test]
fn a_hand_filed_child_reads_back() {
    let mut remembered = LastActive::<5>::new();
    remembered.remember(2, 4);

    assert_eq!(remembered.last_child(2), Some(4));
}

#[test]
fn remembering_nothing_is_the_default() {
    let remembered = LastActive::<5>::default();

    assert_eq!(remembered.last_child(2), None);
}

#[test]
fn a_state_outside_the_chart_records_nothing_and_reads_as_none() {
    let mut history: LastActive<4> = LastActive::new();

    history.remember(99, 1);

    assert_eq!(history.last_child(99), None);
    assert_eq!(history.last_child(1), None);
}
