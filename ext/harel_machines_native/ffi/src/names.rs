//! The names a chart's states were declared with.

use std::collections::HashMap;

use harel_machines::StateId;
use magnus::{Error, RArray, Ruby, Symbol};

/// A chart's state names in declaration order and the index of each.
pub struct Names {
    order: Vec<String>,
    ids: HashMap<String, StateId>,
}

impl Names {
    pub fn new(order: Vec<String>) -> Self {
        let ids = order
            .iter()
            .zip(0..)
            .map(|(name, id)| (name.clone(), id))
            .collect();
        Self { order, ids }
    }

    /// The index of the state called `name`.
    ///
    /// # Errors
    ///
    /// `ArgumentError` if the chart holds no state of that name.
    pub fn id(&self, ruby: &Ruby, name: &str) -> Result<StateId, Error> {
        self.ids.get(name).copied().ok_or_else(|| {
            Error::new(
                ruby.exception_arg_error(),
                format!("unknown state: :{name}"),
            )
        })
    }

    /// The index of the state `symbol` names.
    ///
    /// # Errors
    ///
    /// As [`Names::id`].
    pub fn symbol_id(&self, ruby: &Ruby, symbol: Symbol) -> Result<StateId, Error> {
        self.id(ruby, &symbol.name()?)
    }

    pub fn symbol(&self, ruby: &Ruby, id: StateId) -> Symbol {
        ruby.to_symbol(&self.order[usize::from(id)])
    }

    /// # Errors
    ///
    /// `FrozenError` never: the array is freshly allocated.
    pub fn symbols(&self, ruby: &Ruby, ids: &[StateId]) -> Result<RArray, Error> {
        let array = ruby.ary_new_capa(ids.len());
        for &id in ids {
            array.push(self.symbol(ruby, id))?;
        }
        Ok(array)
    }

    /// Every state name, in declaration order.
    ///
    /// # Errors
    ///
    /// As [`Names::symbols`].
    pub fn all(&self, ruby: &Ruby) -> Result<RArray, Error> {
        let array = ruby.ary_new_capa(self.order.len());
        for name in &self.order {
            array.push(ruby.to_symbol(name))?;
        }
        Ok(array)
    }

    /// Every state index, in declaration order.
    pub fn ids(&self) -> impl Iterator<Item = StateId> {
        (0..).take(self.order.len())
    }

    pub const fn len(&self) -> usize {
        self.order.len()
    }
}
