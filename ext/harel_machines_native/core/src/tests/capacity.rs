//! Charts at exactly `MAX_STATES`, the largest capacity: every index a
//! `StateId` holds is used, so nothing may count past `u16::MAX`. They run
//! on a thread with a stack sized for the scratch arrays.
use std::boxed::Box;
use std::vec::Vec;

use crate::{Chart, MAX_STATES, Refusal, State, StateId, TransitionKind};

fn on_big_stack(body: impl FnOnce() -> Result<(), Refusal> + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(body)
        .unwrap()
        .join()
        .unwrap()
        .unwrap();
}

fn chart(states: &[State]) -> Box<Chart<MAX_STATES>> {
    Box::new(Chart::<MAX_STATES>::new(states).unwrap())
}

#[test]
fn one_state_at_the_largest_capacity() {
    on_big_stack(|| {
        let chart = chart(&[State::atomic(None)]);
        let start = chart.enter(0)?;
        assert_eq!(start.entry(), &[0]);
        assert_eq!(chart.configuration(&[0])?, *start.configuration());
        let again = chart.resolve(start.configuration(), 0, 0, TransitionKind::External)?;
        assert_eq!(again.exit(), &[0]);
        assert_eq!(chart.state(1), Err(Refusal::UnknownState));
        Ok(())
    });
}

#[test]
fn a_full_parallel_chart_at_the_largest_capacity() {
    on_big_stack(|| {
        let mut states = Vec::with_capacity(MAX_STATES);
        states.push(State::parallel(None));
        states.resize(MAX_STATES, State::atomic(Some(0)));
        let chart = chart(&states);
        let last = StateId::try_from(MAX_STATES - 1).unwrap();
        let start = chart.enter(0)?;
        assert_eq!(start.entry().len(), MAX_STATES);
        assert_eq!(start.configuration().len(), MAX_STATES - 1);
        let step = chart.resolve(start.configuration(), last, 1, TransitionKind::External)?;
        assert_eq!(step.exit().len(), MAX_STATES - 1);
        assert_eq!(step.exit().last(), Some(&last));
        let mut history = crate::LastActive::new();
        history.record(&chart, step.exit())?;
        assert_eq!(
            chart.configuration(step.configuration().as_slice())?,
            *step.configuration()
        );
        Ok(())
    });
}

#[test]
fn a_full_chain_at_the_largest_capacity() {
    on_big_stack(|| {
        let last = StateId::try_from(MAX_STATES - 1).unwrap();
        let mut states: Vec<State> = (0..last)
            .map(|id| State::compound(id.checked_sub(1), id + 1))
            .collect();
        states.push(State::atomic(Some(last - 1)));
        let chart = chart(&states);
        assert_eq!(chart.depth(last)?, MAX_STATES - 1);
        let start = chart.enter(last)?;
        assert_eq!(start.entry().len(), MAX_STATES);
        let step = chart.resolve(start.configuration(), 0, 0, TransitionKind::External)?;
        assert_eq!(step.exit().len(), MAX_STATES);
        let mut history = crate::LastActive::new();
        history.record(&chart, step.exit())?;
        assert_eq!(history.remembered().count(), MAX_STATES - 1);
        let saved: Vec<_> = history.remembered().collect();
        assert_eq!(crate::LastActive::restored(&chart, &saved)?, history);
        Ok(())
    });
}
