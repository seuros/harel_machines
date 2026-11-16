//! `HarelMachines::Chart::Declaration`: the states a chart is built from.
use std::cell::RefCell;

use harel_machines::{History, State};
use magnus::scan_args::{get_kwargs, scan_args};
use magnus::typed_data::Obj;
use magnus::{Error, RClass, RHash, Ruby, Value, method, prelude::*};
use matryona_machines::ruby::Symbols;

use crate::convert::{Histories, name, range};
use crate::names::Names;
use crate::shape::SHAPES;

/// What a declared state's children mean, with its initial child still named.
enum Composition {
    Atomic,
    Compound { initial: String },
    Parallel,
}

/// One state as declared.
struct Declared {
    name: String,
    parent: Option<String>,
    composition: Composition,
    history: History,
}

/// The states a chart is built from, in declaration order. Only
/// `Chart.build` makes one, and hands it to its block.
#[magnus::wrap(class = "HarelMachines::Chart::Declaration", free_immediately, size)]
pub struct Declaration(RefCell<Vec<Declared>>);

/// The positional name and the keyword Hash of a declaration call.
fn split(args: &[Value]) -> Result<(Value, RHash), Error> {
    let args = scan_args::<(Value,), (), (), (), RHash, ()>(args)?;
    Ok((args.required.0, args.keywords))
}

/// `parent:` as given: `nil` for the root, otherwise a state's name.
fn parent(ruby: &Ruby, value: Value) -> Result<Option<String>, Error> {
    if value.is_nil() {
        return Ok(None);
    }
    name(ruby, value, "parent").map(Some)
}

/// `history:` as given, `:none` when it is left out.
fn history(ruby: &Ruby, value: Option<Value>) -> Result<History, Error> {
    value.map_or(Ok(History::None), |value| Histories::parse(ruby, value))
}

impl Declaration {
    pub const fn new() -> Self {
        Self(RefCell::new(Vec::new()))
    }

    /// `atomic(name, parent:)`: a leaf.
    fn atomic(ruby: &Ruby, this: Obj<Self>, args: &[Value]) -> Result<(), Error> {
        this.check_frozen()?;
        let (state, keywords) = split(args)?;
        let (parent_value,) =
            get_kwargs::<_, (Value,), (), ()>(keywords, &["parent"], &[])?.required;
        let state = name(ruby, state, "name")?;
        let parent = parent(ruby, parent_value)?;
        Self::push(
            this,
            ruby,
            state,
            parent,
            Composition::Atomic,
            History::None,
        )
    }

    /// `compound(name, parent:, initial:, history: :none)`: one child active
    /// at a time.
    fn compound(ruby: &Ruby, this: Obj<Self>, args: &[Value]) -> Result<(), Error> {
        this.check_frozen()?;
        let (state, keywords) = split(args)?;
        let keywords = get_kwargs::<_, (Value, Value), (Option<Value>,), ()>(
            keywords,
            &["parent", "initial"],
            &["history"],
        )?;
        let (parent_value, initial) = keywords.required;
        let state = name(ruby, state, "name")?;
        let parent = parent(ruby, parent_value)?;
        let initial = name(ruby, initial, "initial")?;
        let history = history(ruby, keywords.optional.0)?;
        Self::push(
            this,
            ruby,
            state,
            parent,
            Composition::Compound { initial },
            history,
        )
    }

    /// `parallel(name, parent:, history: :none)`: every child a region, all
    /// active at once.
    fn parallel(ruby: &Ruby, this: Obj<Self>, args: &[Value]) -> Result<(), Error> {
        this.check_frozen()?;
        let (state, keywords) = split(args)?;
        let keywords =
            get_kwargs::<_, (Value,), (Option<Value>,), ()>(keywords, &["parent"], &["history"])?;
        let state = name(ruby, state, "name")?;
        let parent = parent(ruby, keywords.required.0)?;
        let history = history(ruby, keywords.optional.0)?;
        Self::push(this, ruby, state, parent, Composition::Parallel, history)
    }

    /// Files a state once every argument was read, checking frozen again
    /// first: whatever reading ran, the receiver is still mutable.
    fn push(
        this: Obj<Self>,
        ruby: &Ruby,
        name: String,
        parent: Option<String>,
        composition: Composition,
        history: History,
    ) -> Result<(), Error> {
        this.check_frozen()?;
        let mut declared = this
            .0
            .try_borrow_mut()
            .map_err(|_| Error::new(ruby.exception_runtime_error(), "declared during a build"))?;
        declared.push(Declared {
            name,
            parent,
            composition,
            history,
        });
        Ok(())
    }

    /// The declared states as the core's states, and the names it knows them
    /// by.
    ///
    /// # Errors
    ///
    /// `RangeError` for more states than the largest shape holds (checked
    /// before any name is numbered), a name declared twice, or a parent or an
    /// initial child naming a state that was never declared.
    pub fn resolve(&self, ruby: &Ruby) -> Result<(Names, Vec<State>), Error> {
        let declared = self
            .0
            .try_borrow()
            .map_err(|_| Error::new(ruby.exception_runtime_error(), "declared during a build"))?;
        let largest = SHAPES.iter().copied().max().unwrap_or(0);
        if declared.len() > largest {
            let message = format!(
                "a chart of {} states is not supported, the shapes are {SHAPES:?}",
                declared.len()
            );
            return Err(range(ruby, message));
        }
        let names = Names::new(
            ruby,
            declared.iter().map(|state| state.name.clone()).collect(),
        )?;
        let mut states = Vec::with_capacity(declared.len());
        for state in declared.iter() {
            let parent = state
                .parent
                .as_deref()
                .map(|parent| names.id(ruby, parent))
                .transpose()?;
            let core = match &state.composition {
                Composition::Atomic => State::atomic(parent),
                Composition::Compound { initial } => {
                    State::compound(parent, names.id(ruby, initial)?)
                }
                Composition::Parallel => State::parallel(parent),
            };
            states.push(core.with_history(state.history));
        }
        Ok((names, states))
    }
}

pub fn define(chart: RClass, ruby: &Ruby) -> Result<RClass, Error> {
    let class = chart.define_class("Declaration", ruby.class_object())?;
    class.undef_default_alloc_func();
    class.define_method("atomic", method!(Declaration::atomic, -1))?;
    class.define_method("compound", method!(Declaration::compound, -1))?;
    class.define_method("parallel", method!(Declaration::parallel, -1))?;
    Ok(class)
}
