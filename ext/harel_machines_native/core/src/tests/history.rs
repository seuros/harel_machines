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
