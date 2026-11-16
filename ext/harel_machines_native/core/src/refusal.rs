//! What a chart cannot be built from, and what it does not answer.
use core::fmt;

/// Why a call on a built [`Chart`](crate::Chart) or a
/// [`LastActive`](crate::LastActive) was not honoured. A refused call
/// changes nothing; no call reports "not honoured" any other way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// A state index past the last state of the chart.
    UnknownState,
    /// The configuration is not one this chart can be in: a leaf that is
    /// not atomic or appears twice, a compound state with other than one
    /// active child, or a parallel state with a region left inactive. Only a
    /// configuration built for another chart can be one.
    InvalidConfiguration,
    /// The remembered children are not ones this chart can hold: a child
    /// filed under a state that is not compound, or under a state that does
    /// not contain it.
    InvalidHistory,
    /// The source of the transition is not active in the configuration:
    /// neither one of its leaves nor a state containing one.
    Inactive,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnknownState => "no state of the chart has this index",
            Self::InvalidConfiguration => "the chart cannot be in this configuration",
            Self::InvalidHistory => "the chart cannot have remembered this child",
            Self::Inactive => "the source is not active in the configuration",
        })
    }
}

impl core::error::Error for Refusal {}

/// Why [`Chart::new`](crate::Chart::new) refused the states it was given.
///
/// The checks run in the order of the variants, so a list of states that is
/// wrong in two ways answers the first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartError {
    /// No states at all.
    Empty,
    /// More states than the chart's capacity.
    TooMany,
    /// A parent or an initial child names an index past the last state.
    UnknownState,
    /// An atomic state is named as a parent: entry stops at an atomic state,
    /// so its children could never be entered.
    AtomicParent,
    /// A compound state's initial child is not one of its children.
    InitialNotChild,
    /// A parallel state has no regions, so entering it reaches no leaf.
    EmptyParallel,
    /// An atomic state is marked with history, which it has no child to
    /// restore.
    AtomicHistory,
    /// More than one state has no parent. A chart is one tree, so any two
    /// states share an ancestor that bounds a transition between them.
    MultipleRoots,
    /// A parent chain that never reaches a root: a cycle.
    NotATree,
}

impl fmt::Display for ChartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "a chart needs at least one state",
            Self::TooMany => "more states than the chart holds",
            Self::UnknownState => "a parent or an initial child names no state",
            Self::AtomicParent => "an atomic state cannot contain other states",
            Self::InitialNotChild => "an initial state is not a child of its compound state",
            Self::EmptyParallel => "a parallel state has no regions",
            Self::AtomicHistory => "an atomic state has no history to restore",
            Self::MultipleRoots => "a chart has exactly one root",
            Self::NotATree => "the parent links have a cycle",
        })
    }
}

impl core::error::Error for ChartError {}
