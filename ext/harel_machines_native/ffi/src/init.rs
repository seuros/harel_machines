//! The extension's entry point.
use magnus::{Error, Ruby, prelude::*};
use matryona_machines::ruby::Symbols;

use crate::convert::Refusals;
use crate::shape::SHAPES;

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("HarelMachines")?;
    let shapes = ruby.ary_from_vec(SHAPES.to_vec());
    shapes.freeze();
    module.const_set("SHAPES", shapes)?;
    module.const_set("REFUSALS", Refusals::every(ruby))?;
    let chart = module.define_class("Chart", ruby.class_object())?;
    crate::chart::define(chart)?;
    let declaration = crate::declaration::define(chart, ruby)?;
    let history = crate::history::define(ruby, module)?;
    crate::classes::resolve(ruby, module, chart, declaration, history)
}
