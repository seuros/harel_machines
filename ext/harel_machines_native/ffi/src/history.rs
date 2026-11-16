//! `HarelMachines::History`: the child each compound state of one chart
//! last held.
use std::cell::RefCell;
use std::sync::Arc;

use harel_machines::StateId;
use magnus::typed_data::Obj;
use magnus::{Error, RArray, RModule, Ruby, Symbol, Value, function, method, prelude::*};
use matryona_machines::ruby::{
    exact_array_n_with, exact_array_with, exact_fields, exact_typed, symbol,
};

use crate::chart::{Inner, chart};
use crate::classes::classes;
use crate::convert::{range, reply, states};

/// What one chart's compound states last held, as (state, child) pairs in
/// ascending state order: always a history the core accepts for the chart.
#[magnus::wrap(class = "HarelMachines::History", free_immediately, size)]
pub struct History {
    pub chart: Arc<Inner>,
    remembered: RefCell<Vec<(StateId, StateId)>>,
}

/// The history a Ruby value is: exactly a `HarelMachines::History`,
/// `TypeError` for anything else, a subclass included.
pub fn history(ruby: &Ruby, value: Value) -> Result<Obj<History>, Error> {
    let class = classes(ruby)?.history.get(ruby);
    exact_typed::<History>(ruby, value, class, "expected a HarelMachines::History")
}

/// A `RuntimeError` for a history borrowed while it was being read, which
/// no call following the rules causes.
fn busy(ruby: &Ruby) -> Error {
    Error::new(ruby.exception_runtime_error(), "the history is in use")
}

impl History {
    /// `History.new(chart)`: remembers nothing.
    fn new(ruby: &Ruby, chart_value: Value) -> Result<Self, Error> {
        let chart = chart(ruby, chart_value)?;
        Ok(Self {
            chart: Arc::clone(&chart.inner),
            remembered: RefCell::new(Vec::new()),
        })
    }

    /// `History.restored(chart, pairs)`: the core's `LastActive::restored`.
    ///
    /// # Errors
    ///
    /// `TypeError` for anything but a Chart and an Array of two-Symbol
    /// Arrays; `RangeError` for an unknown name or a history no transitions
    /// over the chart leave.
    fn restored(ruby: &Ruby, chart_value: Value, pairs: Value) -> Result<Self, Error> {
        let chart = chart(ruby, chart_value)?;
        let message = || "remembered must be an Array of [state, child] Symbol pairs";
        let list = exact_array_with(ruby, pairs, message)?;
        let mut read = Vec::with_capacity(list.len());
        for index in 0..list.len() {
            let pair: Value = list.entry(isize::try_from(index).unwrap_or(isize::MAX))?;
            let [state, child] = exact_array_n_with::<2>(ruby, pair, message)?;
            let names = &chart.inner.names;
            read.push((
                names.lookup(ruby, state, "state")?,
                names.lookup(ruby, child, "child")?,
            ));
        }
        let remembered = chart
            .inner
            .shape
            .restored(&read)
            .map_err(|_| range(ruby, "no transitions over this chart leave this history"))?;
        Ok(Self {
            chart: Arc::clone(&chart.inner),
            remembered: RefCell::new(remembered),
        })
    }

    /// What it holds now, copied out under a short borrow.
    pub fn snapshot(&self, ruby: &Ruby) -> Result<Vec<(StateId, StateId)>, Error> {
        self.remembered
            .try_borrow()
            .map(|held| held.clone())
            .map_err(|_| busy(ruby))
    }

    /// `record(transition)`: files its exit set, answering `:recorded` or the
    /// core's refusal. Frozen is checked first and again just before the
    /// core is asked; everything Ruby passed is read in between.
    fn record(ruby: &Ruby, this: Obj<Self>, transition: Value) -> Result<Value, Error> {
        this.check_frozen()?;
        let class = classes(ruby)?.transition.get(ruby);
        let [exit, _, _] = exact_fields::<3>(
            ruby,
            transition,
            class,
            "expected a HarelMachines::Transition",
        )?;
        let exited = states(ruby, &this.chart.names, exit, "exit")?;
        this.check_frozen()?;
        let held = this.snapshot(ruby)?;
        let outcome = this.chart.shape.record(&held, &exited);
        reply(ruby, outcome, |remembered| {
            *this.remembered.try_borrow_mut().map_err(|_| busy(ruby))? = remembered;
            Ok(symbol(ruby, "recorded").as_value())
        })
    }

    /// Every (state, child) pair held, as frozen two-Symbol Arrays in a
    /// frozen Array.
    fn remembered(ruby: &Ruby, this: &Self) -> Result<RArray, Error> {
        let names = &this.chart.names;
        let pairs = ruby.ary_new();
        for pair in this.snapshot(ruby)? {
            pairs.push(names.symbols(ruby, &<[StateId; 2]>::from(pair)))?;
        }
        pairs.freeze();
        Ok(pairs)
    }

    /// `last_child(state)`: the child it held when last exited, or `nil`.
    fn last_child(ruby: &Ruby, this: &Self, state: Value) -> Result<Option<Symbol>, Error> {
        let state = this.chart.names.lookup(ruby, state, "state")?;
        let held = this.snapshot(ruby)?;
        let child = held
            .iter()
            .find(|(parent, _)| *parent == state)
            .map(|(_, child)| *child);
        Ok(child.and_then(|child| this.chart.names.symbol(ruby, child)))
    }

    fn inspect(ruby: &Ruby, this: &Self) -> Result<String, Error> {
        let held = this.snapshot(ruby)?.len();
        Ok(format!("#<HarelMachines::History remembered={held}>"))
    }
}

pub fn define(ruby: &Ruby, module: RModule) -> Result<magnus::RClass, Error> {
    let class = module.define_class("History", ruby.class_object())?;
    class.undef_default_alloc_func();
    class.define_singleton_method("new", function!(History::new, 1))?;
    class.define_singleton_method("restored", function!(History::restored, 2))?;
    class.define_method("record", method!(History::record, 1))?;
    class.define_method("remembered", method!(History::remembered, 0))?;
    class.define_method("last_child", method!(History::last_child, 1))?;
    class.define_method("inspect", method!(History::inspect, 0))?;
    Ok(class)
}
