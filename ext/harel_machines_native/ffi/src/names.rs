//! The names a chart's states were declared with.
use std::collections::HashMap;

use harel_machines::StateId;
use magnus::{Error, RArray, Ruby, Symbol, Value, prelude::*};

use crate::convert::{name, range};

/// A chart's state names in declaration order and the index of each.
pub struct Names {
    order: Vec<String>,
    ids: HashMap<String, StateId>,
}

impl Names {
    /// The names, each given once.
    ///
    /// # Errors
    ///
    /// `RangeError` if a name is declared twice: the duplicate would be
    /// unreachable and every lookup would resolve to the later index.
    pub fn new(ruby: &Ruby, order: Vec<String>) -> Result<Self, Error> {
        let mut ids = HashMap::with_capacity(order.len());
        for (state, id) in order.iter().zip(0..) {
            if ids.insert(state.clone(), id).is_some() {
                return Err(range(ruby, format!("state :{state} is declared twice")));
            }
        }
        Ok(Self { order, ids })
    }

    /// The index of the state called `state`.
    ///
    /// # Errors
    ///
    /// `RangeError` if the chart holds no state of that name.
    pub fn id(&self, ruby: &Ruby, state: &str) -> Result<StateId, Error> {
        self.ids
            .get(state)
            .copied()
            .ok_or_else(|| range(ruby, format!("unknown state: :{state}")))
    }

    /// The index of the state the Symbol `value` names.
    ///
    /// # Errors
    ///
    /// `TypeError` for anything but a Symbol, `RangeError` for a Symbol no
    /// state of the chart was declared with.
    pub fn lookup(&self, ruby: &Ruby, value: Value, what: &str) -> Result<StateId, Error> {
        self.id(ruby, &name(ruby, value, what)?)
    }

    /// The Symbol of the state at `id`, or `None` for an index the chart
    /// does not hold, which only a broken core could answer.
    pub fn symbol(&self, ruby: &Ruby, id: StateId) -> Option<Symbol> {
        self.order
            .get(usize::from(id))
            .map(|state| ruby.to_symbol(state))
    }

    /// The states at `ids` as a frozen Array of Symbols.
    pub fn symbols(&self, ruby: &Ruby, ids: &[StateId]) -> RArray {
        let array = ruby.ary_from_iter(ids.iter().filter_map(|&id| self.symbol(ruby, id)));
        array.freeze();
        array
    }

    /// Every state name, in declaration order, as a frozen Array.
    pub fn all(&self, ruby: &Ruby) -> RArray {
        let array = ruby.ary_from_iter(self.order.iter().map(|state| ruby.to_symbol(state)));
        array.freeze();
        array
    }

    pub const fn len(&self) -> usize {
        self.order.len()
    }
}
