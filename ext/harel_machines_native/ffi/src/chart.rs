//! `HarelMachines::Chart`: a state tree, and the transitions resolved over it.

use std::sync::Arc;

use harel_machines::{StateId, TransitionKind};
use magnus::scan_args::{get_kwargs, scan_args};
use magnus::typed_data::Obj;
use magnus::{Error, RArray, RHash, RModule, Ruby, Symbol, Value, function, method, prelude::*};

use crate::declaration::{self, Declaration};
use crate::history::History;
use crate::names::Names;
use crate::shape::{Resolved, SHAPES, Tree};
use crate::{chart_error, data};

/// The source is exited and entered again.
const EXTERNAL: &str = "external";
/// The source stays active, and so does everything between it and the target.
const INTERNAL: &str = "internal";

/// An immutable state tree and the names its states were declared with.
#[magnus::wrap(class = "HarelMachines::Chart", free_immediately, size)]
pub struct Chart {
    tree: Tree,
    names: Arc<Names>,
}

fn kind(ruby: &Ruby, name: Option<Symbol>) -> Result<TransitionKind, Error> {
    let name = name.map(Symbol::name).transpose()?;
    match name.as_deref() {
        None | Some(EXTERNAL) => Ok(TransitionKind::External),
        Some(INTERNAL) => Ok(TransitionKind::Internal),
        Some(other) => Err(Error::new(
            ruby.exception_arg_error(),
            format!("unknown transition kind: :{other}, expected :{EXTERNAL} or :{INTERNAL}"),
        )),
    }
}

fn unsupported(ruby: &Ruby, count: usize) -> Error {
    Error::new(
        ruby.exception_arg_error(),
        format!("a chart of {count} states is not supported, the shapes are {SHAPES:?}"),
    )
}

impl Chart {
    /// Builds a chart from the states declared in the block.
    ///
    /// # Errors
    ///
    /// `ArgumentError` if a name is unknown or there are more states than the
    /// largest shape holds. `LocalJumpError` without a block.
    fn build(ruby: &Ruby) -> Result<Self, Error> {
        let declaration = ruby.obj_wrap(Declaration::new());
        ruby.block_proc()?.call::<_, Value>((declaration,))?;
        let (names, states) = declaration.resolve(ruby)?;
        let tree = Tree::new(&states).ok_or_else(|| unsupported(ruby, states.len()))?;
        Ok(Self {
            tree,
            names: Arc::new(names),
        })
    }

    pub fn names(&self) -> Arc<Names> {
        Arc::clone(&self.names)
    }

    /// What contains each state, by index.
    pub fn parents(&self) -> Vec<Option<StateId>> {
        self.names.ids().map(|id| self.tree.parent(id)).collect()
    }

    /// Every state name, in declaration order.
    ///
    /// # Errors
    ///
    /// `FrozenError` never: the array is freshly allocated.
    fn states(ruby: &Ruby, rb_self: &Self) -> Result<RArray, Error> {
        rb_self.names.all(ruby)
    }

    /// How many states contain this one.
    ///
    /// # Errors
    ///
    /// `ArgumentError` if the state is unknown.
    fn depth(ruby: &Ruby, rb_self: &Self, state: Symbol) -> Result<usize, Error> {
        let state = rb_self.names.symbol_id(ruby, state)?;
        rb_self
            .tree
            .depth(state)
            .map_err(|error| chart_error(ruby, error))
    }

    /// Whether `ancestor` contains `descendant`.
    ///
    /// # Errors
    ///
    /// `ArgumentError` if either state is unknown.
    fn ancestor(
        ruby: &Ruby,
        rb_self: &Self,
        ancestor: Symbol,
        descendant: Symbol,
    ) -> Result<bool, Error> {
        let ancestor = rb_self.names.symbol_id(ruby, ancestor)?;
        let descendant = rb_self.names.symbol_id(ruby, descendant)?;
        rb_self
            .tree
            .ancestor(ancestor, descendant)
            .map_err(|error| chart_error(ruby, error))
    }

    /// The innermost state that contains or is both of them.
    ///
    /// # Errors
    ///
    /// `ArgumentError` if either state is unknown. `NoCommonAncestor` if they
    /// sit in disjoint trees.
    fn least_common_ancestor(
        ruby: &Ruby,
        rb_self: &Self,
        left: Symbol,
        right: Symbol,
    ) -> Result<Symbol, Error> {
        let left = rb_self.names.symbol_id(ruby, left)?;
        let right = rb_self.names.symbol_id(ruby, right)?;
        rb_self
            .tree
            .least_common_ancestor(left, right)
            .map(|state| rb_self.names.symbol(ruby, state))
            .map_err(|error| chart_error(ruby, error))
    }

    /// Entering `state` from nothing, restoring what `history:` holds.
    ///
    /// # Errors
    ///
    /// `ArgumentError` if the state is unknown.
    fn enter(ruby: &Ruby, rb_self: &Self, args: &[Value]) -> Result<Value, Error> {
        let args = scan_args::<(Symbol,), (), (), (), RHash, ()>(args)?;
        let (history,) =
            get_kwargs::<_, (), (Option<Obj<History>>,), ()>(args.keywords, &[], &["history"])?
                .optional;
        let state = rb_self.names.symbol_id(ruby, args.required.0)?;
        let held = history.as_ref().map(|history| history.children());
        let resolved = rb_self
            .tree
            .enter(state, held.as_deref().unwrap_or(&[]))
            .map_err(|error| chart_error(ruby, error))?;
        rb_self.transition(ruby, &resolved)
    }

    /// Which states a transition from `from:` to `to:` exits and enters.
    ///
    /// # Errors
    ///
    /// `ArgumentError` if a state or the kind is unknown. `NoCommonAncestor` if
    /// source and target sit in disjoint trees.
    fn resolve(ruby: &Ruby, rb_self: &Self, args: &[Value]) -> Result<Value, Error> {
        let args = scan_args::<(RArray,), (), (), (), RHash, ()>(args)?;
        let keywords = get_kwargs::<_, (Symbol, Symbol), (Option<Symbol>, Option<Obj<History>>), ()>(
            args.keywords,
            &["from", "to"],
            &["kind", "history"],
        )?;
        let (from, to) = keywords.required;
        let (marker, history) = keywords.optional;
        let mut leaves = Vec::with_capacity(args.required.0.len());
        for leaf in args.required.0 {
            leaves.push(rb_self.names.symbol_id(ruby, Symbol::try_convert(leaf)?)?);
        }
        let held = history.as_ref().map(|history| history.children());
        let resolved = rb_self
            .tree
            .resolve(
                &leaves,
                rb_self.names.symbol_id(ruby, from)?,
                rb_self.names.symbol_id(ruby, to)?,
                kind(ruby, marker)?,
                held.as_deref().unwrap_or(&[]),
            )
            .map_err(|error| chart_error(ruby, error))?;
        rb_self.transition(ruby, &resolved)
    }

    fn transition(&self, ruby: &Ruby, resolved: &Resolved) -> Result<Value, Error> {
        data::transition(
            ruby,
            self.names.symbols(ruby, &resolved.exit)?,
            self.names.symbols(ruby, &resolved.entry)?,
            self.names.symbols(ruby, &resolved.configuration)?,
        )
    }

    fn inspect(&self) -> String {
        format!("#<HarelMachines::Chart states={}>", self.names.len())
    }
}

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    let class = module.define_class("Chart", ruby.class_object())?;
    class.define_singleton_method("build", function!(Chart::build, 0))?;
    class.define_method("states", method!(Chart::states, 0))?;
    class.define_method("depth", method!(Chart::depth, 1))?;
    class.define_method("ancestor?", method!(Chart::ancestor, 2))?;
    class.define_method(
        "least_common_ancestor",
        method!(Chart::least_common_ancestor, 2),
    )?;
    class.define_method("enter", method!(Chart::enter, -1))?;
    class.define_method("resolve", method!(Chart::resolve, -1))?;
    class.define_method("inspect", method!(Chart::inspect, 0))?;
    declaration::define(ruby, class)
}
