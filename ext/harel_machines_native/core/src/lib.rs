#![no_std]
#![cfg_attr(not(test), deny(clippy::indexing_slicing))]
#![doc = include_str!("../README.md")]

mod buffer;
mod chart;
mod configuration;
mod descend;
mod history;
mod refusal;
mod transition;
mod validate;

pub use chart::{Chart, Composition, State};
pub use configuration::Configuration;
pub use history::{History, LastActive};
pub use refusal::{ChartError, Refusal};
pub use transition::{Transition, TransitionKind};

/// Index of a state in a [`Chart`]: its position in the slice the chart was
/// built from.
pub type StateId = u16;

/// The most states a chart can hold.
///
/// Every index of a chart this size is a [`StateId`], and so is every depth,
/// with `u16::MAX` left over: the chart's validation uses it to mark a depth
/// it has not worked out yet.
pub const MAX_STATES: usize = 65_535;

/// Each item with its index as a [`StateId`], stopping before an index a
/// `StateId` cannot hold. A range counting in `StateId` itself would step
/// past `u16::MAX` once its last index is handed out.
pub(crate) fn indexed<I: IntoIterator>(items: I) -> impl Iterator<Item = (StateId, I::Item)> {
    items
        .into_iter()
        .enumerate()
        .map_while(|(index, item)| StateId::try_from(index).ok().map(|id| (id, item)))
}

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod tests;
