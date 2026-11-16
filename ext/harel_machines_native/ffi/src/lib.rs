//! Ruby bindings for the `harel-machines` statechart hierarchy.
//!
//! State names are Ruby Symbols at this boundary and `u16` indices inside,
//! and a chart holds the mapping between the two. The core is sized by a
//! const generic, so a chart picks the smallest of a fixed set of
//! monomorphised shapes that holds it and raises `RangeError` for more states
//! than the largest.
mod chart;
mod classes;
mod convert;
mod declaration;
mod history;
mod init;
mod names;
mod shape;
