//! Exit and entry sets over the fixed charts.
use super::charts::{leaves, pipeline, player, radio, workflow};
use crate::{Chart, History, LastActive, Refusal, State, TransitionKind};

const EXTERNAL: TransitionKind = TransitionKind::External;
const INTERNAL: TransitionKind = TransitionKind::Internal;

#[test]
fn a_transition_between_siblings_reaches_only_their_shared_parent() -> Result<(), Refusal> {
    let chart = player();
    let step = chart.resolve(&leaves(&chart, &[3]), 3, 4, EXTERNAL)?;

    assert_eq!(step.exit(), &[3]);
    assert_eq!(step.entry(), &[4]);
    assert_eq!(step.configuration().as_slice(), &[4]);
    Ok(())
}

#[test]
fn leaving_a_deeply_nested_state_exits_innermost_first() -> Result<(), Refusal> {
    let chart = pipeline();
    let step = chart.resolve(&leaves(&chart, &[5]), 5, 1, EXTERNAL)?;

    assert_eq!(step.exit(), &[5, 4, 3, 2]);
    assert_eq!(step.entry(), &[1]);
    Ok(())
}

#[test]
fn entering_a_composite_state_descends_through_initial_children_to_a_leaf() -> Result<(), Refusal> {
    let chart = pipeline();
    let step = chart.resolve(&leaves(&chart, &[1]), 1, 2, EXTERNAL)?;

    assert_eq!(step.entry(), &[2, 3, 4, 5]);
    assert_eq!(step.configuration().as_slice(), &[5]);
    Ok(())
}

#[test]
fn entering_from_nothing_enters_every_state_from_the_root_down() -> Result<(), Refusal> {
    let chart = pipeline();

    assert_eq!(chart.enter(0)?.entry(), &[0, 1]);
    let deep = chart.enter(4)?;
    assert_eq!(deep.entry(), &[0, 2, 3, 4, 5]);
    assert_eq!(deep.exit(), &[]);
    assert_eq!(deep.configuration().as_slice(), &[5]);
    Ok(())
}

#[test]
fn entering_a_parallel_state_activates_every_region() -> Result<(), Refusal> {
    let chart = radio(History::None);
    let step = chart.resolve(&leaves(&chart, &[1]), 1, 2, EXTERNAL)?;

    assert_eq!(step.entry(), &[2, 3, 4, 6, 7]);
    assert_eq!(step.configuration().as_slice(), &[4, 7]);
    Ok(())
}

#[test]
fn leaving_a_parallel_state_exits_every_region_innermost_first() -> Result<(), Refusal> {
    let chart = radio(History::None);
    let step = chart.resolve(&leaves(&chart, &[4, 7]), 2, 1, EXTERNAL)?;

    assert_eq!(step.exit(), &[4, 7, 3, 6, 2]);
    assert_eq!(step.configuration().as_slice(), &[1]);
    Ok(())
}

#[test]
fn a_move_inside_one_region_leaves_the_other_alone() -> Result<(), Refusal> {
    let chart = radio(History::None);
    let step = chart.resolve(&leaves(&chart, &[4, 7]), 4, 5, EXTERNAL)?;

    assert_eq!(step.exit(), &[4]);
    assert_eq!(step.entry(), &[5]);
    assert_eq!(step.configuration().as_slice(), &[5, 7]);
    Ok(())
}

/// Enters `Work`, moves to `Tcp`, then leaves `Work` again, so the chart has
/// a child to remember at two depths.
fn park(chart: &Chart<7>) -> Result<(LastActive<7>, crate::Configuration<7>), Refusal> {
    let mut remembered = LastActive::new();
    let start = chart.enter(0)?;
    let work = chart.resolve(start.configuration(), 1, 2, EXTERNAL)?;
    remembered.record(chart, work.exit())?;
    let tcp = chart.resolve(work.configuration(), 4, 5, EXTERNAL)?;
    remembered.record(chart, tcp.exit())?;
    let idle = chart.resolve(tcp.configuration(), 2, 1, EXTERNAL)?;
    remembered.record(chart, idle.exit())?;
    Ok((remembered, *idle.configuration()))
}

#[test]
fn without_history_re_entry_goes_back_to_the_initial_child() -> Result<(), Refusal> {
    let chart = workflow(History::None);
    let (remembered, idle) = park(&chart)?;
    let step = chart.resolve_with_history(&idle, 1, 2, EXTERNAL, &remembered)?;

    assert_eq!(step.entry(), &[2, 3, 4]);
    Ok(())
}

#[test]
fn shallow_history_restores_the_last_child_and_no_deeper() -> Result<(), Refusal> {
    let chart = workflow(History::Shallow);
    let (remembered, idle) = park(&chart)?;
    let step = chart.resolve_with_history(&idle, 1, 2, EXTERNAL, &remembered)?;

    assert_eq!(step.entry(), &[2, 3, 4]);
    assert_eq!(remembered.last_child(3), Some(5));
    Ok(())
}

#[test]
fn deep_history_restores_the_configuration_that_was_active() -> Result<(), Refusal> {
    let chart = workflow(History::Deep);
    let (remembered, idle) = park(&chart)?;
    let step = chart.resolve_with_history(&idle, 1, 2, EXTERNAL, &remembered)?;

    assert_eq!(step.entry(), &[2, 3, 5]);
    assert_eq!(step.configuration().as_slice(), &[5]);
    assert_eq!(
        chart.enter_with_history(2, &remembered)?.entry(),
        &[0, 2, 3, 5]
    );
    Ok(())
}

#[test]
fn deep_history_on_a_parallel_state_restores_every_region() -> Result<(), Refusal> {
    let chart = radio(History::Deep);
    let mut remembered = LastActive::new();
    let loud = chart.resolve(&leaves(&chart, &[4, 7]), 4, 5, EXTERNAL)?;
    remembered.record(&chart, loud.exit())?;
    let offline = chart.resolve(loud.configuration(), 7, 8, EXTERNAL)?;
    remembered.record(&chart, offline.exit())?;
    assert_eq!(offline.configuration().as_slice(), &[5, 8]);
    let off = chart.resolve(offline.configuration(), 2, 1, EXTERNAL)?;
    remembered.record(&chart, off.exit())?;

    let on = chart.resolve_with_history(off.configuration(), 1, 2, EXTERNAL, &remembered)?;

    assert_eq!(on.entry(), &[2, 3, 5, 6, 8]);
    assert_eq!(on.configuration().as_slice(), &[5, 8]);
    Ok(())
}

#[test]
fn an_internal_self_transition_exits_nothing() -> Result<(), Refusal> {
    let chart = player();
    let step = chart.resolve(&leaves(&chart, &[3]), 3, 3, INTERNAL)?;

    assert_eq!(step.exit(), []);
    assert_eq!(step.entry(), []);
    assert_eq!(step.configuration().as_slice(), &[3]);
    Ok(())
}

#[test]
fn an_external_self_transition_exits_and_enters_the_source() -> Result<(), Refusal> {
    let chart = player();
    let step = chart.resolve(&leaves(&chart, &[3]), 3, 3, EXTERNAL)?;

    assert_eq!(step.exit(), &[3]);
    assert_eq!(step.entry(), &[3]);
    assert_eq!(step.configuration().as_slice(), &[3]);
    Ok(())
}

#[test]
fn a_transition_to_an_ancestor_exits_the_source_but_not_the_ancestor() -> Result<(), Refusal> {
    let chart = pipeline();
    let step = chart.resolve(&leaves(&chart, &[5]), 5, 2, EXTERNAL)?;

    assert_eq!(step.exit(), &[5, 4, 3]);
    assert_eq!(step.entry(), &[3, 4, 5]);
    Ok(())
}

#[test]
fn only_an_external_transition_into_a_descendant_exits_the_source() -> Result<(), Refusal> {
    let chart = pipeline();
    let inside = leaves(&chart, &[5]);

    let internal = chart.resolve(&inside, 2, 4, INTERNAL)?;
    assert_eq!(internal.exit(), &[5, 4, 3]);
    assert_eq!(internal.entry(), &[3, 4, 5]);

    let external = chart.resolve(&inside, 2, 4, EXTERNAL)?;
    assert_eq!(external.exit(), &[5, 4, 3, 2]);
    assert_eq!(external.entry(), &[2, 3, 4, 5]);
    Ok(())
}

#[test]
fn the_least_common_ancestor_is_the_innermost_container_of_both() -> Result<(), Refusal> {
    let chart = pipeline();
    assert_eq!(chart.least_common_ancestor(5, 2)?, 2);
    assert_eq!(chart.least_common_ancestor(2, 5)?, 2);
    assert_eq!(chart.least_common_ancestor(1, 5)?, 0);
    assert_eq!(chart.least_common_ancestor(3, 3)?, 3);
    Ok(())
}

#[test]
fn depth_counts_the_states_that_contain_one() -> Result<(), Refusal> {
    let chart = pipeline();
    assert_eq!(chart.depth(0)?, 0);
    assert_eq!(chart.depth(5)?, 4);
    assert!(chart.is_ancestor(2, 5)?);
    assert!(!chart.is_ancestor(5, 2)?);
    assert!(!chart.is_ancestor(5, 5)?);
    assert!(!chart.is_ancestor(1, 5)?);
    assert_eq!(chart.root(), 0);
    assert_eq!(chart.parent(3)?, Some(2));
    assert_eq!(chart.parent(0)?, None);
    Ok(())
}

#[test]
fn an_external_self_transition_on_the_root_re_enters_the_whole_chart() -> Result<(), Refusal> {
    let chart = player();
    let step = chart.resolve(&leaves(&chart, &[4]), 0, 0, EXTERNAL)?;

    assert_eq!(step.exit(), &[4, 2, 0]);
    assert_eq!(step.entry(), &[0, 1]);
    assert_eq!(step.configuration().as_slice(), &[1]);
    Ok(())
}

#[test]
fn an_external_transition_from_the_root_into_itself_exits_the_root() -> Result<(), Refusal> {
    let chart = player();
    let step = chart.resolve(&leaves(&chart, &[1]), 0, 2, EXTERNAL)?;

    assert_eq!(step.exit(), &[1, 0]);
    assert_eq!(step.entry(), &[0, 2, 3]);
    assert_eq!(step.configuration().as_slice(), &[3]);
    Ok(())
}

#[test]
fn targeting_one_region_still_activates_the_others() -> Result<(), Refusal> {
    let chart = radio(History::None);
    let step = chart.resolve(&leaves(&chart, &[1]), 1, 4, EXTERNAL)?;

    assert_eq!(step.entry(), &[2, 3, 4, 6, 7]);
    assert_eq!(step.configuration().as_slice(), &[4, 7]);
    Ok(())
}

#[test]
fn an_external_self_transition_on_a_region_leaves_its_siblings_untouched() -> Result<(), Refusal> {
    let chart = radio(History::None);
    let step = chart.resolve(&leaves(&chart, &[5, 8]), 6, 6, EXTERNAL)?;

    assert_eq!(step.exit(), &[8, 6]);
    assert_eq!(step.entry(), &[6, 7]);
    assert_eq!(step.configuration().as_slice(), &[5, 7]);
    Ok(())
}

#[test]
fn an_external_self_transition_on_a_parallel_state_re_enters_every_region() -> Result<(), Refusal> {
    let chart = radio(History::None);
    let step = chart.resolve(&leaves(&chart, &[5, 8]), 2, 2, EXTERNAL)?;

    assert_eq!(step.exit(), &[5, 8, 3, 6, 2]);
    assert_eq!(step.entry(), &[2, 3, 4, 6, 7]);
    assert_eq!(step.configuration().as_slice(), &[4, 7]);
    Ok(())
}

#[test]
fn a_cross_region_transition_keeps_every_region_active() -> Result<(), Refusal> {
    let chart = radio(History::None);
    let step = chart.resolve(&leaves(&chart, &[4, 7]), 4, 7, EXTERNAL)?;

    assert_eq!(step.configuration().as_slice(), &[4, 7]);
    Ok(())
}

/// A chain as deep as the capacity allows: entry and exit walk it without
/// recursion, so the depth costs nothing but the scratch arrays.
#[test]
fn the_deepest_chain_enters_and_exits_without_recursion() -> Result<(), Refusal> {
    const DEPTH: usize = 4096;
    let mut states = std::vec::Vec::with_capacity(DEPTH);
    states.push(State::compound(None, 1));
    for id in 1..DEPTH - 1 {
        let id = u16::try_from(id).unwrap();
        states.push(State::compound(Some(id - 1), id + 1));
    }
    states.push(State::atomic(Some(u16::try_from(DEPTH - 2).unwrap())));
    let chart = std::boxed::Box::new(Chart::<DEPTH>::new(&states).unwrap());
    let leaf = u16::try_from(DEPTH - 1).unwrap();

    let start = chart.enter(0)?;
    assert_eq!(start.entry().len(), DEPTH);
    assert_eq!(chart.depth(leaf)?, DEPTH - 1);
    let again = chart.resolve(start.configuration(), leaf, leaf, EXTERNAL)?;
    assert_eq!(again.exit(), &[leaf]);
    let again = chart.resolve(start.configuration(), 0, 0, EXTERNAL)?;
    assert_eq!(again.exit().len(), DEPTH);
    assert_eq!(again.exit().first(), Some(&leaf));
    assert_eq!(again.exit().last(), Some(&0));
    Ok(())
}
