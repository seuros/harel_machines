//! The classes the binding builds and reads, resolved once at init and
//! pinned. The exact-class reads are `matryona_machines::ruby`'s.
use magnus::{Error, RClass, RModule, Ruby};
use matryona_machines::ruby::Class;

use crate::chart::Chart;
use crate::declaration::Declaration;
use crate::history::History;

/// The classes resolved at init.
pub struct Classes {
    pub transition: Class,
    pub chart: Class,
    pub history: Class,
}

matryona_machines::registry! {
    static CLASSES: Classes = "harel_machines";
    /// The classes resolved at init.
    pub fn classes;
}

/// Resolves the `Transition` class `lib/harel_machines.rb` defined before it
/// loaded the extension and pins the three classes init defined, fixing
/// magnus's class for each wrapped type to them: building one later runs no
/// `Object.const_get`. Called once, from init.
pub fn resolve(
    ruby: &Ruby,
    module: RModule,
    chart: RClass,
    declaration: RClass,
    history: RClass,
) -> Result<(), Error> {
    Class::pin_wrapped::<Declaration>(ruby, declaration)?;
    let classes = Classes {
        transition: Class::resolve(ruby, module, "Transition")?,
        chart: Class::pin_wrapped::<Chart>(ruby, chart)?,
        history: Class::pin_wrapped::<History>(ruby, history)?,
    };
    CLASSES.set(ruby, classes).map(drop)
}
