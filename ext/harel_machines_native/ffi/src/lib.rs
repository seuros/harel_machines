//! Ruby bindings for the `harel-machines` statechart hierarchy.
//!
//! State names are Ruby symbols at this boundary and `u16` indices inside, and
//! a chart holds the mapping between the two. The tree itself is sized by a
//! const generic, so a chart dispatches its state count to a fixed set of
//! monomorphised shapes and raises `ArgumentError` for any count above the
//! largest.

#![allow(clippy::needless_pass_by_value)]

mod chart;
mod data;
mod declaration;
mod history;
mod names;
mod shape;

use std::sync::OnceLock;

use harel_machines::Error as ChartError;
use magnus::value::Opaque;
use magnus::{Error, ExceptionClass, RModule, Ruby, prelude::*};

use crate::shape::SHAPES;

/// `HarelMachines::NoCommonAncestor`, defined in [`init`].
static NO_COMMON_ANCESTOR: OnceLock<Opaque<ExceptionClass>> = OnceLock::new();
/// `HarelMachines::EmptyParallel`, defined in [`init`].
static EMPTY_PARALLEL: OnceLock<Opaque<ExceptionClass>> = OnceLock::new();
/// `HarelMachines::NotATree`, defined in [`init`].
static NOT_A_TREE: OnceLock<Opaque<ExceptionClass>> = OnceLock::new();

pub(crate) fn uninitialised(ruby: &Ruby) -> Error {
    Error::new(
        ruby.exception_runtime_error(),
        "the harel_machines extension is not initialised",
    )
}

pub(crate) fn defined_twice(ruby: &Ruby, name: &str) -> Error {
    Error::new(
        ruby.exception_runtime_error(),
        format!("HarelMachines::{name} was initialised twice"),
    )
}

/// The `HarelMachines::Error` subclass a core error is raised as.
pub(crate) fn chart_error(ruby: &Ruby, error: ChartError) -> Error {
    let class = match error {
        ChartError::NoCommonAncestor => &NO_COMMON_ANCESTOR,
        ChartError::EmptyParallel => &EMPTY_PARALLEL,
        ChartError::NotATree => &NOT_A_TREE,
    };
    class.get().map_or_else(
        || uninitialised(ruby),
        |class| Error::new(ruby.get_inner(*class), error.to_string()),
    )
}

fn define_errors(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    let base = module.define_error("Error", ruby.exception_standard_error())?;
    for (name, slot) in [
        ("NoCommonAncestor", &NO_COMMON_ANCESTOR),
        ("EmptyParallel", &EMPTY_PARALLEL),
        ("NotATree", &NOT_A_TREE),
    ] {
        let class = module.define_error(name, base)?;
        slot.set(class.into())
            .map_err(|_| defined_twice(ruby, name))?;
    }
    Ok(())
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("HarelMachines")?;
    let shapes = ruby.ary_from_vec(SHAPES.to_vec());
    shapes.freeze();
    module.const_set("SHAPES", shapes)?;
    define_errors(ruby, module)?;
    data::define(ruby, module)?;
    chart::define(ruby, module)?;
    history::define(ruby, module)
}
