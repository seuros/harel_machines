//! Readers for what Ruby passes in, the Symbol tables, and the one answer
//! built from scratch: a `Transition`.
use std::borrow::Cow;

use harel_machines::{History, Refusal, StateId, TransitionKind};
use magnus::{Error, RArray, RClass, RStruct, Ruby, Symbol, Value, prelude::*};
use matryona_machines::ruby::{class_name, exact_array_n_with, exact_array_with};

use crate::names::Names;
use crate::shape::Resolved;

matryona_machines::symbols! {
    /// The core's refusals as Symbols: the snake case of each variant.
    pub Refusals for Refusal as "a refusal" {
        UnknownState => "unknown_state",
        InvalidConfiguration => "invalid_configuration",
        InvalidHistory => "invalid_history",
        Inactive => "inactive",
    }
}

matryona_machines::symbols! {
    /// The core's transition kinds as Symbols.
    pub Kinds for TransitionKind as "kind" {
        External => "external",
        Internal => "internal",
    }
}

matryona_machines::symbols! {
    /// The core's history markers as Symbols.
    pub Histories for History as "history" {
        None => "none",
        Shallow => "shallow",
        Deep => "deep",
    }
}

/// `RangeError` with `message`.
pub fn range(ruby: &Ruby, message: impl Into<Cow<'static, str>>) -> Error {
    Error::new(ruby.exception_range_error(), message)
}

/// The text of exactly a Symbol, read natively: `TypeError` for anything
/// else (a String included), `RangeError` for a Symbol whose name is not
/// valid UTF-8, which no state can be declared with.
pub fn name(ruby: &Ruby, value: Value, what: &str) -> Result<String, Error> {
    let Some(symbol) = Symbol::from_value(value) else {
        let message = format!("{what} must be a Symbol, not {}", class_name(value));
        return Err(Error::new(ruby.exception_type_error(), message));
    };
    symbol.name().map(Cow::into_owned).map_err(|_| {
        range(
            ruby,
            format!("{what} must be a Symbol whose name is valid UTF-8"),
        )
    })
}

/// The states an exactly-`Array` of Symbols names, in its order: `TypeError`
/// for anything but an Array (a subclass included) or a non-Symbol element,
/// `RangeError` for a name the chart does not hold. Elements are read
/// natively; no Ruby method of the array runs.
pub fn states(ruby: &Ruby, names: &Names, value: Value, what: &str) -> Result<Vec<StateId>, Error> {
    let array = exact_array_with(ruby, value, || {
        format!("{what} must be an Array of Symbols")
    })?;
    let mut ids = Vec::with_capacity(array.len());
    for index in 0..array.len() {
        let element: Value = array.entry(isize::try_from(index).unwrap_or(isize::MAX))?;
        ids.push(names.lookup(ruby, element, what)?);
    }
    Ok(ids)
}

/// The three lists a marshalled `Transition` is loaded from, checked and
/// copied: exactly an Array of three exactly-Arrays, each of distinct
/// Symbols, read natively. Answers a frozen Array of three frozen copies.
/// `TypeError` for any other shape or a non-Symbol, `RangeError` for a name
/// listed twice in one list: no transition exits or enters a state twice,
/// and none settles in a leaf twice.
pub fn transition_fields(ruby: &Ruby, value: Value) -> Result<RArray, Error> {
    let message = "a Transition loads from [exit, entry, configuration], three Arrays of Symbols";
    let lists = exact_array_n_with::<3>(ruby, value, || message)?;
    let fields = ruby.ary_new_capa(3);
    for (list, field) in lists.into_iter().zip(["exit", "entry", "configuration"]) {
        let array = exact_array_with(ruby, list, || message)?;
        let copy = ruby.ary_new_capa(array.len());
        let mut seen = std::collections::HashSet::with_capacity(array.len());
        for index in 0..array.len() {
            let element: Value = array.entry(isize::try_from(index).unwrap_or(isize::MAX))?;
            let state = name(ruby, element, field)?;
            if !seen.insert(state) {
                return Err(range(ruby, format!("{field} names a state twice")));
            }
            copy.push(element)?;
        }
        copy.freeze();
        fields.push(copy)?;
    }
    fields.freeze();
    Ok(fields)
}

/// A frozen `HarelMachines::Transition` over three frozen Arrays of state
/// names. It is allocated and filled natively, so building it runs no Ruby.
pub fn transition(
    ruby: &Ruby,
    class: RClass,
    names: &Names,
    resolved: &Resolved,
) -> Result<Value, Error> {
    let instance = RStruct::from_value(class.obj_alloc()?.as_value()).ok_or_else(|| {
        Error::new(
            ruby.exception_type_error(),
            "Transition is not a Data class",
        )
    })?;
    let fields: [RArray; 3] = [
        names.symbols(ruby, &resolved.exit),
        names.symbols(ruby, &resolved.entry),
        names.symbols(ruby, &resolved.configuration),
    ];
    for (index, field) in fields.into_iter().enumerate() {
        instance.aset(index, field)?;
    }
    instance.freeze();
    Ok(instance.as_value())
}

/// The refusal's Symbol, or what `build` makes of the honoured answer.
pub fn reply<T>(
    ruby: &Ruby,
    outcome: Result<T, Refusal>,
    build: impl FnOnce(T) -> Result<Value, Error>,
) -> Result<Value, Error> {
    use matryona_machines::ruby::Symbols;
    outcome.map_or_else(
        |refusal| Ok(Refusals::symbol(ruby, refusal).as_value()),
        build,
    )
}
