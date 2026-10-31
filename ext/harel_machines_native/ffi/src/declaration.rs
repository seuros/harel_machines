//! `HarelMachines::Chart::Declaration`: the states a chart is built from.

use std::borrow::Cow;
use std::cell::RefCell;

use harel_machines::{History, State};
use magnus::scan_args::{get_kwargs, scan_args};
use magnus::{Error, RClass, RHash, Ruby, Symbol, Value, method, prelude::*};

use crate::names::Names;

/// Restores the child a composite state last held.
const SHALLOW: &str = "shallow";
/// Restores the whole configuration a composite state last held.
const DEEP: &str = "deep";

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

/// The states a chart is built from, in declaration order.
#[magnus::wrap(class = "HarelMachines::Chart::Declaration", free_immediately, size)]
pub struct Declaration(RefCell<Vec<Declared>>);

fn history(ruby: &Ruby, name: Option<Symbol>) -> Result<History, Error> {
    let name = name.map(Symbol::name).transpose()?;
    match name.as_deref() {
        None => Ok(History::None),
        Some(SHALLOW) => Ok(History::Shallow),
        Some(DEEP) => Ok(History::Deep),
        Some(other) => Err(Error::new(
            ruby.exception_arg_error(),
            format!("unknown history: :{other}, expected :{SHALLOW} or :{DEEP}"),
        )),
    }
}

impl Default for Declaration {
    fn default() -> Self {
        Self::new()
    }
}

impl Declaration {
    pub const fn new() -> Self {
        Self(RefCell::new(Vec::new()))
    }

    /// A leaf.
    fn state(&self, args: &[Value]) -> Result<(), Error> {
        let args = scan_args::<(Symbol,), (), (), (), RHash, ()>(args)?;
        let (parent,) =
            get_kwargs::<_, (), (Option<Symbol>,), ()>(args.keywords, &[], &["parent"])?.optional;
        self.push(args.required.0, parent, Composition::Atomic, History::None)
    }

    /// A state with one child active at a time.
    fn compound(ruby: &Ruby, rb_self: &Self, args: &[Value]) -> Result<(), Error> {
        let args = scan_args::<(Symbol,), (), (), (), RHash, ()>(args)?;
        let keywords = get_kwargs::<_, (Symbol,), (Option<Symbol>, Option<Symbol>), ()>(
            args.keywords,
            &["initial"],
            &["parent", "history"],
        )?;
        let (initial,) = keywords.required;
        let (parent, marker) = keywords.optional;
        rb_self.push(
            args.required.0,
            parent,
            Composition::Compound {
                initial: initial.name()?.into_owned(),
            },
            history(ruby, marker)?,
        )
    }

    /// A state whose children are regions, all active at once.
    fn parallel(ruby: &Ruby, rb_self: &Self, args: &[Value]) -> Result<(), Error> {
        let args = scan_args::<(Symbol,), (), (), (), RHash, ()>(args)?;
        let (parent, marker) = get_kwargs::<_, (), (Option<Symbol>, Option<Symbol>), ()>(
            args.keywords,
            &[],
            &["parent", "history"],
        )?
        .optional;
        rb_self.push(
            args.required.0,
            parent,
            Composition::Parallel,
            history(ruby, marker)?,
        )
    }

    fn push(
        &self,
        name: Symbol,
        parent: Option<Symbol>,
        composition: Composition,
        history: History,
    ) -> Result<(), Error> {
        self.0.borrow_mut().push(Declared {
            name: name.name()?.into_owned(),
            parent: parent
                .map(|parent| parent.name().map(Cow::into_owned))
                .transpose()?,
            composition,
            history,
        });
        Ok(())
    }

    /// The declared states as the core's tree, and the names it knows them by.
    ///
    /// # Errors
    ///
    /// `ArgumentError` if a parent or an initial child names a state that was
    /// never declared.
    pub fn resolve(&self, ruby: &Ruby) -> Result<(Names, Vec<State>), Error> {
        let declared = self.0.borrow();
        let names = Names::new(declared.iter().map(|state| state.name.clone()).collect());
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

pub fn define(ruby: &Ruby, chart: RClass) -> Result<(), Error> {
    let class = chart.define_class("Declaration", ruby.class_object())?;
    class.define_method("state", method!(Declaration::state, -1))?;
    class.define_method("compound", method!(Declaration::compound, -1))?;
    class.define_method("parallel", method!(Declaration::parallel, -1))?;
    Ok(())
}
