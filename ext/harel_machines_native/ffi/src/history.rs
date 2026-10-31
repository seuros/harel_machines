//! `HarelMachines::History`: the child each composite state last held.

use std::cell::{Ref, RefCell};
use std::sync::Arc;

use harel_machines::StateId;
use magnus::typed_data::Obj;
use magnus::{Error, RModule, Ruby, Symbol, Value, function, method, prelude::*};

use crate::chart::Chart;
use crate::data;
use crate::names::Names;

/// One slot per state, holding the child it last held.
#[magnus::wrap(class = "HarelMachines::History", free_immediately, size)]
pub struct History {
    names: Arc<Names>,
    parents: Vec<Option<StateId>>,
    children: RefCell<Vec<Option<StateId>>>,
}

impl History {
    fn new(chart: &Chart) -> Self {
        let parents = chart.parents();
        Self {
            names: chart.names(),
            children: RefCell::new(vec![None; parents.len()]),
            parents,
        }
    }

    /// Files every state the transition exited under the state that contains
    /// it.
    ///
    /// # Errors
    ///
    /// `TypeError` unless given a `HarelMachines::Transition`. `ArgumentError`
    /// if it names a state this chart does not hold.
    fn record(ruby: &Ruby, rb_self: Obj<Self>, transition: Value) -> Result<Obj<Self>, Error> {
        for exited in data::exited(transition)? {
            let exited = rb_self
                .names
                .symbol_id(ruby, Symbol::try_convert(exited)?)?;
            if let Some(parent) = rb_self.parents[usize::from(exited)] {
                rb_self.children.borrow_mut()[usize::from(parent)] = Some(exited);
            }
        }
        Ok(rb_self)
    }

    pub fn children(&self) -> Ref<'_, [Option<StateId>]> {
        Ref::map(self.children.borrow(), Vec::as_slice)
    }

    fn inspect(&self) -> String {
        let remembered = self.children.borrow().iter().flatten().count();
        format!("#<HarelMachines::History remembered={remembered}>")
    }
}

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    let class = module.define_class("History", ruby.class_object())?;
    class.define_singleton_method("new", function!(History::new, 1))?;
    class.define_method("record", method!(History::record, 1))?;
    class.define_method("inspect", method!(History::inspect, 0))?;
    Ok(())
}
