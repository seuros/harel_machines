#![cfg_attr(not(feature = "std"), no_std)]

//! Statechart hierarchy: states inside states.
//!
//! A flat finite state machine answers "given this state and this event, what
//! is the next state". It has no answer for a state that contains other states.
//! This crate is that answer and nothing else.
//!
//! A [`Chart`] is a tree of states over `u16` indices. [`Chart::resolve`] takes
//! a source and a target and returns a [`Transition`]: which states must be
//! exited, in which order, which must be entered, in which order, and the
//! [`Configuration`] of active leaves that results.
//!
//! Nothing here fires an event, evaluates a guard, runs a callback, reads a
//! clock or stores user data. The caller owns the machine and asks this crate
//! which states to leave and which to join.

/// Compiles and runs the README's examples as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

mod buffer;

pub mod chart;
pub mod configuration;
pub mod history;
pub mod transition;

pub use chart::{Chart, Composition, State};
pub use configuration::Configuration;
pub use history::{History, LastActive};
pub use transition::{Transition, TransitionKind};

/// Index of a state in a [`Chart`].
pub type StateId = u16;

/// The most states a chart can hold.
///
/// A state is addressed by a [`StateId`], and scanning for the regions of a
/// parallel state has to step one index past the last state, so the highest
/// `u16` stays unused.
pub const MAX_STATES: usize = 65_535;

/// What a malformed tree cannot answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The source and the target share no ancestor, so nothing bounds the exit
    /// and entry sets. Two states in disjoint trees.
    NoCommonAncestor,
    /// A parallel state has no regions, so entering it reaches no leaf.
    EmptyParallel,
    /// A walk visited more states than the chart holds, which only a parent
    /// cycle or an initial child pointing back up can produce.
    NotATree,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::NoCommonAncestor => "source and target share no ancestor",
            Self::EmptyParallel => "parallel state has no regions",
            Self::NotATree => "the state tree has a cycle",
        };
        f.write_str(message)
    }
}

impl core::error::Error for Error {}
