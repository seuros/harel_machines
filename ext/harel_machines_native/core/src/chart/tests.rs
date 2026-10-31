use crate::{Chart, Configuration, Error, History, LastActive, State, TransitionKind};

/// ```text
/// 0 Player
/// |- 1 Stopped
/// `- 2 Playing
///    |- 3 Buffering
///    `- 4 Streaming
/// ```
const PLAYER: Chart<5> = Chart::new([
    State::compound(None, 1),
    State::atomic(Some(0)),
    State::compound(Some(0), 3),
    State::atomic(Some(2)),
    State::atomic(Some(2)),
]);

/// ```text
/// 0 Root
/// |- 1 Idle
/// `- 2 Work
///    `- 3 Load
///       `- 4 Parse
///          `- 5 Token
/// ```
const PIPELINE: Chart<6> = Chart::new([
    State::compound(None, 1),
    State::atomic(Some(0)),
    State::compound(Some(0), 3),
    State::compound(Some(2), 4),
    State::compound(Some(3), 5),
    State::atomic(Some(4)),
]);

/// ```text
/// 0 Root
/// |- 1 Idle
/// `- 2 Work
///    |- 3 Fetch
///    |  |- 4 Dns
///    |  `- 5 Tcp
///    `- 6 Done
/// ```
const fn workflow(history: History) -> Chart<7> {
    Chart::new([
        State::compound(None, 1),
        State::atomic(Some(0)),
        State::compound(Some(0), 3).with_history(history),
        State::compound(Some(2), 4),
        State::atomic(Some(3)),
        State::atomic(Some(3)),
        State::atomic(Some(2)),
    ])
}

/// ```text
/// 0 Radio
/// |- 1 Off
/// `- 2 On (parallel)
///    |- 3 Volume
///    |  |- 4 Muted
///    |  `- 5 Loud
///    `- 6 Link
///       |- 7 Online
///       `- 8 Offline
/// ```
const fn radio(history: History) -> Chart<9> {
    Chart::new([
        State::compound(None, 1),
        State::atomic(Some(0)),
        State::parallel(Some(0)).with_history(history),
        State::compound(Some(2), 4),
        State::atomic(Some(3)),
        State::atomic(Some(3)),
        State::compound(Some(2), 7),
        State::atomic(Some(6)),
        State::atomic(Some(6)),
    ])
}

fn configuration<const STATES: usize>(leaves: &[u16]) -> Configuration<STATES> {
    let mut configuration = Configuration::new();
    for &leaf in leaves {
        configuration.insert(leaf);
    }
    configuration
}

/// Enters `Work`, moves to `Tcp`, then leaves `Work` again, so the chart has a
/// child to remember at two depths.
fn park(chart: &Chart<7>) -> Result<(LastActive<7>, Configuration<7>), Error> {
    let mut remembered = LastActive::new();

    let start = chart.enter(0)?;
    let work = chart.resolve(start.configuration(), 1, 2, TransitionKind::External)?;
    remembered.record(chart, &work);

    let tcp = chart.resolve(work.configuration(), 4, 5, TransitionKind::External)?;
    remembered.record(chart, &tcp);

    let idle = chart.resolve(tcp.configuration(), 2, 1, TransitionKind::External)?;
    remembered.record(chart, &idle);

    Ok((remembered, *idle.configuration()))
}

#[test]
fn a_transition_between_siblings_reaches_only_their_shared_parent() -> Result<(), Error> {
    let step = PLAYER.resolve(&configuration(&[3]), 3, 4, TransitionKind::External)?;

    assert_eq!(step.exit(), &[3]);
    assert_eq!(step.entry(), &[4]);
    assert_eq!(step.configuration().as_slice(), &[4]);
    Ok(())
}

#[test]
fn leaving_a_deeply_nested_state_exits_innermost_first() -> Result<(), Error> {
    let step = PIPELINE.resolve(&configuration(&[5]), 5, 1, TransitionKind::External)?;

    assert_eq!(step.exit(), &[5, 4, 3, 2]);
    assert_eq!(step.entry(), &[1]);
    Ok(())
}

#[test]
fn entering_a_composite_state_descends_through_initial_children_to_a_leaf() -> Result<(), Error> {
    let step = PIPELINE.resolve(&configuration(&[1]), 1, 2, TransitionKind::External)?;

    assert_eq!(step.entry(), &[2, 3, 4, 5]);
    assert_eq!(step.configuration().as_slice(), &[5]);
    Ok(())
}

#[test]
fn entering_a_parallel_state_activates_every_region() -> Result<(), Error> {
    let chart = radio(History::None);

    let step = chart.resolve(&configuration(&[1]), 1, 2, TransitionKind::External)?;

    assert_eq!(step.entry(), &[2, 3, 4, 6, 7]);
    assert_eq!(step.configuration().as_slice(), &[4, 7]);
    Ok(())
}

#[test]
fn leaving_a_parallel_state_exits_every_region_innermost_first() -> Result<(), Error> {
    let chart = radio(History::None);

    let step = chart.resolve(&configuration(&[4, 7]), 2, 1, TransitionKind::External)?;

    assert_eq!(step.exit(), &[4, 7, 3, 6, 2]);
    assert_eq!(step.configuration().as_slice(), &[1]);
    Ok(())
}

#[test]
fn a_move_inside_one_region_leaves_the_other_alone() -> Result<(), Error> {
    let chart = radio(History::None);

    let step = chart.resolve(&configuration(&[4, 7]), 4, 5, TransitionKind::External)?;

    assert_eq!(step.exit(), &[4]);
    assert_eq!(step.entry(), &[5]);
    assert_eq!(step.configuration().as_slice(), &[5, 7]);
    Ok(())
}

#[test]
fn without_history_re_entry_goes_back_to_the_initial_child() -> Result<(), Error> {
    let chart = workflow(History::None);
    let (remembered, idle) = park(&chart)?;

    let step = chart.resolve_with_history(&idle, 1, 2, TransitionKind::External, &remembered)?;

    assert_eq!(step.entry(), &[2, 3, 4]);
    Ok(())
}

#[test]
fn shallow_history_restores_the_last_child_and_no_deeper() -> Result<(), Error> {
    let chart = workflow(History::Shallow);
    let (remembered, idle) = park(&chart)?;

    let step = chart.resolve_with_history(&idle, 1, 2, TransitionKind::External, &remembered)?;

    assert_eq!(step.entry(), &[2, 3, 4]);
    assert_eq!(remembered.last_child(3), Some(5));
    Ok(())
}

#[test]
fn deep_history_restores_the_configuration_that_was_active() -> Result<(), Error> {
    let chart = workflow(History::Deep);
    let (remembered, idle) = park(&chart)?;

    let step = chart.resolve_with_history(&idle, 1, 2, TransitionKind::External, &remembered)?;

    assert_eq!(step.entry(), &[2, 3, 5]);
    assert_eq!(step.configuration().as_slice(), &[5]);
    Ok(())
}

#[test]
fn deep_history_on_a_parallel_state_restores_every_region() -> Result<(), Error> {
    let chart = radio(History::Deep);
    let mut remembered = LastActive::new();

    let loud = chart.resolve(&configuration(&[4, 7]), 4, 5, TransitionKind::External)?;
    remembered.record(&chart, &loud);
    let offline = chart.resolve(loud.configuration(), 7, 8, TransitionKind::External)?;
    remembered.record(&chart, &offline);
    assert_eq!(offline.configuration().as_slice(), &[5, 8]);

    let off = chart.resolve(offline.configuration(), 2, 1, TransitionKind::External)?;
    remembered.record(&chart, &off);

    let on = chart.resolve_with_history(
        off.configuration(),
        1,
        2,
        TransitionKind::External,
        &remembered,
    )?;

    assert_eq!(on.entry(), &[2, 3, 5, 6, 8]);
    assert_eq!(on.configuration().as_slice(), &[5, 8]);
    Ok(())
}

#[test]
fn an_internal_self_transition_exits_nothing() -> Result<(), Error> {
    let step = PLAYER.resolve(&configuration(&[3]), 3, 3, TransitionKind::Internal)?;

    assert_eq!(step.exit(), []);
    assert_eq!(step.entry(), []);
    assert_eq!(step.configuration().as_slice(), &[3]);
    Ok(())
}

#[test]
fn an_external_self_transition_exits_and_enters_the_source() -> Result<(), Error> {
    let step = PLAYER.resolve(&configuration(&[3]), 3, 3, TransitionKind::External)?;

    assert_eq!(step.exit(), &[3]);
    assert_eq!(step.entry(), &[3]);
    assert_eq!(step.configuration().as_slice(), &[3]);
    Ok(())
}

#[test]
fn a_transition_to_an_ancestor_exits_the_source_but_not_the_ancestor() -> Result<(), Error> {
    let step = PIPELINE.resolve(&configuration(&[5]), 5, 2, TransitionKind::External)?;

    assert_eq!(step.exit(), &[5, 4, 3]);
    assert_eq!(step.entry(), &[3, 4, 5]);
    Ok(())
}

#[test]
fn only_an_external_transition_into_a_descendant_exits_the_source() -> Result<(), Error> {
    let inside = configuration(&[5]);

    let internal = PIPELINE.resolve(&inside, 2, 4, TransitionKind::Internal)?;
    assert_eq!(internal.exit(), &[5, 4, 3]);
    assert_eq!(internal.entry(), &[3, 4, 5]);

    let external = PIPELINE.resolve(&inside, 2, 4, TransitionKind::External)?;
    assert_eq!(external.exit(), &[5, 4, 3, 2]);
    assert_eq!(external.entry(), &[2, 3, 4, 5]);
    Ok(())
}

#[test]
fn the_least_common_ancestor_is_the_innermost_container_of_both() -> Result<(), Error> {
    assert_eq!(PIPELINE.least_common_ancestor(5, 2)?, 2);
    assert_eq!(PIPELINE.least_common_ancestor(2, 5)?, 2);
    assert_eq!(PIPELINE.least_common_ancestor(1, 5)?, 0);
    assert_eq!(PIPELINE.least_common_ancestor(3, 3)?, 3);
    Ok(())
}

#[test]
fn depth_counts_the_states_that_contain_one() -> Result<(), Error> {
    assert_eq!(PIPELINE.depth(0)?, 0);
    assert_eq!(PIPELINE.depth(5)?, 4);
    assert!(PIPELINE.is_ancestor(2, 5)?);
    assert!(!PIPELINE.is_ancestor(5, 2)?);
    assert!(!PIPELINE.is_ancestor(5, 5)?);
    Ok(())
}

#[test]
fn states_in_disjoint_trees_share_no_ancestor() {
    const FOREST: Chart<2> = Chart::new([State::atomic(None), State::atomic(None)]);

    assert_eq!(
        FOREST.least_common_ancestor(0, 1),
        Err(Error::NoCommonAncestor)
    );
}

#[test]
fn an_external_transition_cannot_exit_a_root() {
    assert_eq!(
        PLAYER.resolve(&configuration(&[1]), 0, 2, TransitionKind::External),
        Err(Error::NoCommonAncestor)
    );
}

#[test]
fn a_parent_cycle_is_not_a_tree() {
    const KNOT: Chart<2> = Chart::new([State::atomic(Some(1)), State::atomic(Some(0))]);

    assert_eq!(KNOT.depth(0), Err(Error::NotATree));
}

#[test]
fn an_initial_child_that_points_at_its_own_parent_is_not_a_tree() {
    const KNOT: Chart<1> = Chart::new([State::compound(None, 0)]);

    assert_eq!(KNOT.enter(0), Err(Error::NotATree));
}

#[test]
fn a_parallel_state_without_regions_cannot_be_entered() {
    const BARREN: Chart<1> = Chart::new([State::parallel(None)]);

    assert_eq!(BARREN.enter(0), Err(Error::EmptyParallel));
}

#[test]
fn targeting_one_region_still_activates_the_others() -> Result<(), Error> {
    let chart = radio(History::None);

    let step = chart.resolve(&configuration(&[1]), 1, 4, TransitionKind::External)?;

    assert_eq!(step.entry(), &[2, 3, 4, 6, 7]);
    assert_eq!(step.configuration().as_slice(), &[4, 7]);
    Ok(())
}

#[test]
fn a_cross_region_transition_keeps_every_region_active() -> Result<(), Error> {
    let chart = radio(History::None);

    let step = chart.resolve(&configuration(&[4, 7]), 4, 7, TransitionKind::External)?;

    assert_eq!(step.configuration().as_slice(), &[4, 7]);
    Ok(())
}
