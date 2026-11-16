//! Recording, saving and restoring what compound states last held.
use super::charts::workflow;
use crate::{Chart, History, LastActive, Refusal, State, TransitionKind};

/// ```text
/// 0 Root
/// |- 1 Idle
/// `- 2 Work (shallow)
///    |- 3 Fetch
///    `- 4 Done
/// ```
fn shallow() -> Chart<5> {
    Chart::new(&[
        State::compound(None, 1),
        State::atomic(Some(0)),
        State::compound(Some(0), 3).with_history(History::Shallow),
        State::atomic(Some(2)),
        State::atomic(Some(2)),
    ])
    .unwrap()
}

#[test]
fn recording_files_every_exited_state_under_the_compound_state_that_held_it() -> Result<(), Refusal>
{
    let chart = shallow();
    let done = chart.configuration(&[4])?;
    let step = chart.resolve(&done, 2, 1, TransitionKind::External)?;
    let mut remembered = LastActive::new();
    remembered.record(&chart, step.exit())?;

    assert_eq!(remembered.last_child(2), Some(4));
    assert_eq!(remembered.last_child(0), Some(2));
    assert_eq!(remembered.last_child(4), None);
    assert_eq!(remembered.last_child(900), None);
    Ok(())
}

#[test]
fn a_parallel_state_remembers_nothing_of_its_own() -> Result<(), Refusal> {
    let chart = super::charts::radio(History::Deep);
    let on = chart.enter(2)?;
    let off = chart.resolve(on.configuration(), 2, 1, TransitionKind::External)?;
    let mut remembered = LastActive::new();
    remembered.record(&chart, off.exit())?;

    assert_eq!(remembered.last_child(2), None);
    assert_eq!(remembered.last_child(3), Some(4));
    assert_eq!(remembered.last_child(0), Some(2));
    Ok(())
}

#[test]
fn remembering_nothing_is_the_default() {
    assert_eq!(LastActive::<5>::default(), LastActive::new());
    assert_eq!(LastActive::<5>::new().remembered().count(), 0);
}

#[test]
fn a_saved_history_restores_to_the_same_history() -> Result<(), Refusal> {
    let chart = workflow(History::Deep);
    let mut remembered = LastActive::new();
    let start = chart.enter(5)?;
    let idle = chart.resolve(start.configuration(), 2, 1, TransitionKind::External)?;
    remembered.record(&chart, idle.exit())?;
    let saved: std::vec::Vec<_> = remembered.remembered().collect();

    assert_eq!(saved, [(0, 2), (2, 3), (3, 5)]);
    assert_eq!(LastActive::restored(&chart, &saved)?, remembered);
    let mut reversed = saved;
    reversed.reverse();
    assert_eq!(LastActive::restored(&chart, &reversed)?, remembered);
    Ok(())
}

/// Specified by hand, not by the model: an explicitly targeted state is
/// entered as named, so the deep history of an ancestor it passes through
/// restores nothing below it.
///
/// ```text
/// 0 Root (deep, initial Off)
/// |- 1 Off
/// `- 2 Work (initial First, no history)
///    |- 3 First
///    `- 4 Last
/// ```
#[test]
fn an_ancestor_s_history_does_not_reach_into_an_explicit_target() -> Result<(), Refusal> {
    let chart = Chart::<5>::new(&[
        State::compound(None, 1).with_history(History::Deep),
        State::atomic(Some(0)),
        State::compound(Some(0), 3),
        State::atomic(Some(2)),
        State::atomic(Some(2)),
    ])
    .unwrap();
    let last = chart.configuration(&[4])?;
    let leave = chart.resolve(&last, 2, 1, TransitionKind::External)?;
    let mut remembered = LastActive::new();
    remembered.record(&chart, leave.exit())?;
    assert_eq!(remembered.last_child(2), Some(4));

    let back = chart.resolve_with_history(
        leave.configuration(),
        1,
        2,
        TransitionKind::External,
        &remembered,
    )?;
    assert_eq!(back.entry(), &[2, 3]);
    assert_eq!(
        chart.enter_with_history(2, &remembered)?.entry(),
        &[0, 2, 3]
    );
    // The root itself, entered by default, does restore what it held.
    assert_eq!(
        chart.enter_with_history(0, &remembered)?.entry(),
        &[0, 2, 4]
    );
    Ok(())
}

/// A compound root with two atomic children, and a radio: exit sets no
/// transition produces are refused, each with nothing filed.
#[test]
fn record_refuses_exit_sets_no_transition_produces() -> Result<(), Refusal> {
    let pair = Chart::<3>::new(&[
        State::compound(None, 1),
        State::atomic(Some(0)),
        State::atomic(Some(0)),
    ])
    .unwrap();
    let radio = super::charts::radio(History::None);
    let mut remembered = LastActive::new();
    let refused = Err(Refusal::InvalidHistory);

    // Two children of one compound state were never active together.
    assert_eq!(remembered.record(&pair, &[1, 2]), refused);
    // A state named twice.
    assert_eq!(remembered.record(&pair, &[1, 1]), refused);
    // A compound state exited without the child it held.
    assert_eq!(remembered.record(&pair, &[0]), refused);
    let mut radio_history = LastActive::new();
    // A parallel state exited without every region, a region without its leaf.
    assert_eq!(radio_history.record(&radio, &[4, 3, 2]), refused);
    assert_eq!(radio_history.record(&radio, &[4, 7, 3]), refused);
    assert_eq!(radio_history.record(&radio, &[6]), refused);
    // The right set in the wrong order: parents before children, or leaves
    // out of order.
    assert_eq!(radio_history.record(&radio, &[2, 3, 6, 4, 7]), refused);
    assert_eq!(radio_history.record(&radio, &[7, 4, 3, 6, 2]), refused);
    assert_eq!(remembered, LastActive::new());
    assert_eq!(radio_history, LastActive::new());

    // What a transition does produce is filed.
    remembered.record(&pair, &[1])?;
    radio_history.record(&radio, &[4, 7, 3, 6, 2])?;
    radio_history.record(&radio, &[4, 7, 3, 6])?;
    radio_history.record(&radio, &[])?;
    assert_eq!(remembered.last_child(0), Some(1));
    assert_eq!(radio_history.last_child(3), Some(4));
    Ok(())
}
