//! The frozen `Data` object a resolved transition comes back as.

use std::sync::OnceLock;

use magnus::value::Opaque;
use magnus::{Error, RArray, RClass, RModule, RStruct, Ruby, Value, prelude::*};

use crate::{defined_twice, uninitialised};

/// The states to leave, innermost first.
const EXIT: &str = "exit";
/// The states to join, outermost first.
const ENTRY: &str = "entry";
/// The leaves active once the transition has been taken.
const CONFIGURATION: &str = "configuration";

static TRANSITION: OnceLock<Opaque<RClass>> = OnceLock::new();

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    let transition = ruby.define_data(None, (EXIT, ENTRY, CONFIGURATION))?;
    module.const_set("Transition", transition)?;
    TRANSITION
        .set(transition.into())
        .map_err(|_| defined_twice(ruby, "Transition"))
}

/// A `HarelMachines::Transition` over three arrays of state names.
///
/// # Errors
///
/// `RuntimeError` if the extension never initialised.
pub fn transition(
    ruby: &Ruby,
    exit: RArray,
    entry: RArray,
    configuration: RArray,
) -> Result<Value, Error> {
    let class = TRANSITION
        .get()
        .map(|class| ruby.get_inner(*class))
        .ok_or_else(|| uninitialised(ruby))?;
    let instance = RStruct::try_convert(class.obj_alloc()?)?;
    for (index, member) in [exit, entry, configuration].into_iter().enumerate() {
        instance.aset(index, member)?;
    }
    instance.freeze();
    Ok(instance.as_value())
}

/// The states a transition left.
///
/// # Errors
///
/// `TypeError` if `transition` is not a `HarelMachines::Transition`.
pub fn exited(transition: Value) -> Result<RArray, Error> {
    RStruct::try_convert(transition)?.getmember(EXIT)
}
