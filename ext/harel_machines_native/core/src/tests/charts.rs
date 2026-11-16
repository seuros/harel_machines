//! The charts the scenario tests walk.
use crate::{Chart, Configuration, History, State, StateId};

/// ```text
/// 0 Player
/// |- 1 Stopped
/// `- 2 Playing
///    |- 3 Buffering
///    `- 4 Streaming
/// ```
pub fn player() -> Chart<5> {
    Chart::new(&[
        State::compound(None, 1),
        State::atomic(Some(0)),
        State::compound(Some(0), 3),
        State::atomic(Some(2)),
        State::atomic(Some(2)),
    ])
    .unwrap()
}

/// ```text
/// 0 Root
/// |- 1 Idle
/// `- 2 Work
///    `- 3 Load
///       `- 4 Parse
///          `- 5 Token
/// ```
pub fn pipeline() -> Chart<6> {
    Chart::new(&[
        State::compound(None, 1),
        State::atomic(Some(0)),
        State::compound(Some(0), 3),
        State::compound(Some(2), 4),
        State::compound(Some(3), 5),
        State::atomic(Some(4)),
    ])
    .unwrap()
}

/// ```text
/// 0 Root
/// |- 1 Idle
/// `- 2 Work
///    |- 3 Fetch
///    |  |- 4 Dns
///    |  `- 5 Tcp
///    `- 6 Done
/// ```
pub fn workflow(history: History) -> Chart<7> {
    Chart::new(&[
        State::compound(None, 1),
        State::atomic(Some(0)),
        State::compound(Some(0), 3).with_history(history),
        State::compound(Some(2), 4),
        State::atomic(Some(3)),
        State::atomic(Some(3)),
        State::atomic(Some(2)),
    ])
    .unwrap()
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
pub fn radio(history: History) -> Chart<9> {
    Chart::new(&[
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
    .unwrap()
}

/// The configuration of `chart` holding `leaves`.
pub fn leaves<const N: usize>(chart: &Chart<N>, leaves: &[StateId]) -> Configuration<N> {
    chart.configuration(leaves).unwrap()
}
