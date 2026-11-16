//! `HarelMachines::Chart`: a checked state tree, and the transitions resolved
//! over it.
use std::sync::Arc;

use harel_machines::StateId;
use magnus::typed_data::Obj;
use magnus::{Error, RArray, RClass, Ruby, Symbol, Value, function, method, prelude::*};
use matryona_machines::ruby::{Symbols, exact_typed};

use crate::classes::classes;
use crate::convert::{Kinds, range, reply, states, transition};
use crate::declaration::Declaration;
use crate::names::Names;
use crate::shape::{Resolved, SHAPES, Shape, Unbuilt, build};

/// What a chart is: the core at its shape and the names of its states.
/// Shared with every `History` of the chart.
pub struct Inner {
    pub shape: Box<dyn Shape>,
    pub names: Names,
}

/// An immutable state tree and the names its states were declared with.
#[magnus::wrap(class = "HarelMachines::Chart", free_immediately, size)]
pub struct Chart {
    pub inner: Arc<Inner>,
}

/// The chart a Ruby value is: exactly a `HarelMachines::Chart`, `TypeError`
/// for anything else, a subclass included.
pub fn chart(ruby: &Ruby, value: Value) -> Result<Obj<Chart>, Error> {
    let class = classes(ruby)?.chart.get(ruby);
    exact_typed::<Chart>(ruby, value, class, "expected a HarelMachines::Chart")
}

impl Chart {
    /// `Chart.build { |declaration| ... }`.
    ///
    /// # Errors
    ///
    /// `LocalJumpError` without a block; `RangeError` for a name declared
    /// twice, an unknown parent or initial child, more states than the largest
    /// shape, or states the core refuses as a chart.
    fn build(ruby: &Ruby) -> Result<Self, Error> {
        let declaration = ruby.obj_wrap(Declaration::new());
        ruby.yield_value::<_, Value>(declaration)?;
        let (names, states) = declaration.resolve(ruby)?;
        let shape = build(&states).map_err(|unbuilt| match unbuilt {
            Unbuilt::Unsupported => range(
                ruby,
                format!(
                    "a chart of {} states is not supported, the shapes are {SHAPES:?}",
                    states.len()
                ),
            ),
            Unbuilt::Refused(error) => range(ruby, format!("not a chart: {error}")),
        })?;
        Ok(Self {
            inner: Arc::new(Inner { shape, names }),
        })
    }

    fn names(&self) -> &Names {
        &self.inner.names
    }

    fn shape(&self) -> &dyn Shape {
        self.inner.shape.as_ref()
    }

    /// A state's index from its Symbol.
    fn id(&self, ruby: &Ruby, value: Value, what: &str) -> Result<StateId, Error> {
        self.names().lookup(ruby, value, what)
    }

    /// The state at `id` as its Symbol; an index the chart lacks is a broken
    /// core, answered as `RangeError` rather than a panic.
    fn symbol(&self, ruby: &Ruby, id: StateId) -> Result<Symbol, Error> {
        self.names()
            .symbol(ruby, id)
            .ok_or_else(|| range(ruby, "the core answered a state the chart lacks"))
    }

    /// A query the chart answers for every state it holds.
    fn query<T>(ruby: &Ruby, answer: Result<T, harel_machines::Refusal>) -> Result<T, Error> {
        answer.map_err(|refusal| range(ruby, refusal.to_string()))
    }

    fn states(ruby: &Ruby, this: &Self) -> RArray {
        this.names().all(ruby)
    }

    fn size(&self) -> usize {
        self.names().len()
    }

    fn root(ruby: &Ruby, this: &Self) -> Result<Symbol, Error> {
        this.symbol(ruby, this.shape().root())
    }

    fn parent(ruby: &Ruby, this: &Self, state: Value) -> Result<Option<Symbol>, Error> {
        let state = this.id(ruby, state, "state")?;
        let parent = Self::query(ruby, this.shape().parent(state))?;
        parent.map(|parent| this.symbol(ruby, parent)).transpose()
    }

    fn depth(ruby: &Ruby, this: &Self, state: Value) -> Result<usize, Error> {
        let state = this.id(ruby, state, "state")?;
        Self::query(ruby, this.shape().depth(state))
    }

    fn ancestor(
        ruby: &Ruby,
        this: &Self,
        ancestor: Value,
        descendant: Value,
    ) -> Result<bool, Error> {
        let ancestor = this.id(ruby, ancestor, "ancestor")?;
        let descendant = this.id(ruby, descendant, "descendant")?;
        Self::query(ruby, this.shape().is_ancestor(ancestor, descendant))
    }

    fn least_common_ancestor(
        ruby: &Ruby,
        this: &Self,
        left: Value,
        right: Value,
    ) -> Result<Symbol, Error> {
        let (left, right) = (this.id(ruby, left, "left")?, this.id(ruby, right, "right")?);
        let common = Self::query(ruby, this.shape().least_common_ancestor(left, right))?;
        this.symbol(ruby, common)
    }

    /// The leaves of `value`, checked as a configuration of this chart:
    /// `RangeError` for one it cannot be in.
    fn leaves(&self, ruby: &Ruby, value: Value) -> Result<Vec<StateId>, Error> {
        let leaves = states(ruby, self.names(), value, "configuration")?;
        self.shape()
            .configuration(&leaves)
            .map_err(|_| range(ruby, "configuration is not one this chart can be in"))
    }

    /// `configuration(leaves)`: the leaves as the chart holds them, in
    /// declaration order, frozen.
    fn configuration(ruby: &Ruby, this: &Self, value: Value) -> Result<RArray, Error> {
        let leaves = this.leaves(ruby, value)?;
        Ok(this.names().symbols(ruby, &leaves))
    }

    /// The answer to a resolution: its `Transition`, or the refusal's Symbol.
    fn answer(
        &self,
        ruby: &Ruby,
        outcome: Result<Resolved, harel_machines::Refusal>,
    ) -> Result<Value, Error> {
        let class = classes(ruby)?.transition.get(ruby);
        reply(ruby, outcome, |resolved| {
            transition(ruby, class, self.names(), &resolved)
        })
    }

    /// The history `value` holds for this chart: exactly a `History` of this
    /// chart, `TypeError` for anything else, `RangeError` for another chart's.
    fn remembered(&self, ruby: &Ruby, value: Value) -> Result<Vec<(StateId, StateId)>, Error> {
        let history = crate::history::history(ruby, value)?;
        if !Arc::ptr_eq(&history.chart, &self.inner) {
            return Err(range(ruby, "history was recorded on another chart"));
        }
        history.snapshot(ruby)
    }

    fn enter(ruby: &Ruby, this: &Self, target: Value) -> Result<Value, Error> {
        let target = this.id(ruby, target, "target")?;
        this.answer(ruby, this.shape().enter(target, &[]))
    }

    fn enter_with_history(
        ruby: &Ruby,
        this: &Self,
        target: Value,
        history: Value,
    ) -> Result<Value, Error> {
        let target = this.id(ruby, target, "target")?;
        let remembered = this.remembered(ruby, history)?;
        this.answer(ruby, this.shape().enter(target, &remembered))
    }

    fn resolve(
        ruby: &Ruby,
        this: &Self,
        configuration: Value,
        source: Value,
        target: Value,
        kind: Value,
    ) -> Result<Value, Error> {
        Self::resolve_with(ruby, this, [configuration, source, target, kind], None)
    }

    fn resolve_with_history(
        ruby: &Ruby,
        this: &Self,
        configuration: Value,
        source: Value,
        target: Value,
        kind: Value,
        history: Value,
    ) -> Result<Value, Error> {
        Self::resolve_with(
            ruby,
            this,
            [configuration, source, target, kind],
            Some(history),
        )
    }

    /// Every argument read in order, then the core asked once.
    fn resolve_with(
        ruby: &Ruby,
        this: &Self,
        [configuration, source, target, kind]: [Value; 4],
        history: Option<Value>,
    ) -> Result<Value, Error> {
        let leaves = this.leaves(ruby, configuration)?;
        let source = this.id(ruby, source, "source")?;
        let target = this.id(ruby, target, "target")?;
        let kind = Kinds::parse(ruby, kind)?;
        let remembered = match history {
            Some(history) => this.remembered(ruby, history)?,
            None => Vec::new(),
        };
        let outcome = this
            .shape()
            .resolve(&leaves, source, target, kind, &remembered);
        this.answer(ruby, outcome)
    }

    fn inspect(&self) -> String {
        format!("#<HarelMachines::Chart states={}>", self.names().len())
    }
}

pub fn define(class: RClass) -> Result<(), Error> {
    class.undef_default_alloc_func();
    class.define_singleton_method("build", function!(Chart::build, 0))?;
    class.define_method("states", method!(Chart::states, 0))?;
    class.define_method("size", method!(Chart::size, 0))?;
    class.define_method("root", method!(Chart::root, 0))?;
    class.define_method("parent", method!(Chart::parent, 1))?;
    class.define_method("depth", method!(Chart::depth, 1))?;
    class.define_method("ancestor?", method!(Chart::ancestor, 2))?;
    class.define_method(
        "least_common_ancestor",
        method!(Chart::least_common_ancestor, 2),
    )?;
    class.define_method("configuration", method!(Chart::configuration, 1))?;
    class.define_method("enter", method!(Chart::enter, 1))?;
    class.define_method("enter_with_history", method!(Chart::enter_with_history, 2))?;
    class.define_method("resolve", method!(Chart::resolve, 4))?;
    class.define_method(
        "resolve_with_history",
        method!(Chart::resolve_with_history, 5),
    )?;
    class.define_method("inspect", method!(Chart::inspect, 0))?;
    Ok(())
}
