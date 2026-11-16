# harel-machines

Statechart hierarchy: states inside states. `no_std`, allocation-free, zero
dependencies.

David Harel's 1987 statecharts added to the flat finite state machine the one
thing it never had: a state can contain other states. A flat machine answers
"given this state and this event, what is the next state". It has no answer for
"what happens to the five states I was inside of".

This crate is that answer and nothing else. It holds no events, no guards, no
callbacks, no DSL and no data. The caller owns the machine (`state_machines`
already owns the flat part) and asks here which states to leave and which to
join.

## The contract

- `Chart::<STATES>::new(&[State]) -> Result<Chart, ChartError>` builds one
  checked tree of at most `STATES` states (a `STATES` above `MAX_STATES`,
  65 535, does not compile). The index of a state in the slice is its
  `StateId`. It refuses, in this order: no states (`Empty`), more than
  `STATES` (`TooMany`), a parent or initial child past the last state
  (`UnknownState`), an atomic state with children (`AtomicParent`), an initial
  child that is not the compound state's own (`InitialNotChild`), a parallel
  state without regions (`EmptyParallel`), history on an atomic state
  (`AtomicHistory`), more than one root (`MultipleRoots`), a cycle
  (`NotATree`). Every chart that exists is one tree, so any two states have a
  least common ancestor.
- Every other call that can refuse returns `Result<T, Refusal>`; `T` says only
  what was honoured, and a refused call changes nothing. `Refusal` is
  `UnknownState` (an index past the last state), `InvalidConfiguration`,
  `InvalidHistory` (a configuration or history this chart cannot be in: only
  one built for another chart can be) and `Inactive` (the source of a
  transition is not active). No public enum is `#[non_exhaustive]`: a new
  variant is a breaking change every caller matches on.
- Queries: `len`, `states`, `root`, `state`, `parent`, `depth`, `is_ancestor`,
  `least_common_ancestor`. They read the chart and nothing else.
- `enter(target)` and `enter_with_history(target, &LastActive)` enter from
  nothing: every state from the root down to `target`, then whatever `target`
  contains, to a leaf in every region opened.
- `resolve(&Configuration, source, target, TransitionKind)` and
  `resolve_with_history(.., &LastActive)` answer the exit set (innermost
  first), the entry set (outermost first) and the configuration they settle
  in. They refuse `UnknownState`, `InvalidConfiguration`, `InvalidHistory`,
  `Inactive`, checked in that order.
- A `Configuration` comes only from a chart: `Configuration::new()` (nothing
  entered), a transition's `configuration()`, or `Chart::configuration(&[leaves])`,
  which restores a saved one and accepts exactly the configurations the chart
  can be in (no leaves, or atomic leaves each named once with one active child
  under every active compound state and every region of every active parallel
  state).
- `LastActive` is the history: the child each compound state held when last
  exited. `record(&chart, transition.exit())` files an exit set,
  `remembered()` lists what it holds and `LastActive::restored(&chart, &pairs)`
  rebuilds it. Both `record` and `restored` accept exactly what some sequence of
  recorded transitions over the chart leaves: a remembered child that is
  compound remembers a child too (it was exited with everything below it), a
  remembered parallel state has every compound region remembering one, and a
  compound state that remembers nothing has something remembered below at most
  one of its children. A refused `record` files nothing.

The scratch space of a call is a handful of `STATES`-entry arrays on the
stack, and every walk is iterative: depth costs no recursion, but a chart near
`MAX_STATES` wants a stack of a few megabytes.

## A transition is an exit set and an entry set

Give it a tree and a transition. It gives back the states to exit, innermost
first, and the states to enter, outermost first, bounded by the least common
ancestor of source and target.

```rust
use harel_machines::{Chart, State, TransitionKind};

// 0 Call
// |- 1 Idle
// `- 2 Active
//    |- 3 Dialing
//    `- 4 Talking
let call = Chart::<5>::new(&[
    State::compound(None, 1),
    State::atomic(Some(0)),
    State::compound(Some(0), 3),
    State::atomic(Some(2)),
    State::atomic(Some(2)),
])?;

// Entering a composite state means entering one of its children, so entry
// keeps descending until it reaches a leaf.
let start = call.enter(0)?;
assert_eq!(start.entry(), &[0, 1]);
assert_eq!(start.configuration().as_slice(), &[1]);

let dial = call.resolve(start.configuration(), 1, 2, TransitionKind::External)?;
assert_eq!(dial.exit(), &[1]);
assert_eq!(dial.entry(), &[2, 3]);

// Siblings never disturb the parent that holds them.
let talk = call.resolve(dial.configuration(), 3, 4, TransitionKind::External)?;
assert_eq!(talk.exit(), &[3]);
assert_eq!(talk.entry(), &[4]);

// Hanging up from inside leaves everything it was inside of, innermost first.
let hang_up = call.resolve(talk.configuration(), 4, 1, TransitionKind::External)?;
assert_eq!(hang_up.exit(), &[4, 2]);
assert_eq!(hang_up.entry(), &[1]);
assert_eq!(hang_up.configuration().as_slice(), &[1]);

// Entering a nested state from nothing enters everything above it too.
assert_eq!(call.enter(4)?.entry(), &[0, 2, 4]);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Parallel regions make the configuration a set

A state whose children are regions is in all of them at once, so what a chart
is "in" is a set of leaves, not one leaf.

```rust
use harel_machines::{Chart, Refusal, State, TransitionKind};

// 0 Radio
// |- 1 Off
// `- 2 On (parallel)
//    |- 3 Volume  -> 4 Muted | 5 Loud
//    `- 6 Link    -> 7 Online | 8 Offline
let radio = Chart::<9>::new(&[
    State::compound(None, 1),
    State::atomic(Some(0)),
    State::parallel(Some(0)),
    State::compound(Some(2), 4),
    State::atomic(Some(3)),
    State::atomic(Some(3)),
    State::compound(Some(2), 7),
    State::atomic(Some(6)),
    State::atomic(Some(6)),
])?;

let on = radio.resolve(radio.enter(0)?.configuration(), 1, 2, TransitionKind::External)?;
assert_eq!(on.entry(), &[2, 3, 4, 6, 7]);
assert_eq!(on.configuration().as_slice(), &[4, 7]);

// Turning the volume up says nothing about the link.
let loud = radio.resolve(on.configuration(), 4, 5, TransitionKind::External)?;
assert_eq!(loud.configuration().as_slice(), &[5, 7]);

// Turning the radio off leaves both regions, children before parents.
let off = radio.resolve(loud.configuration(), 2, 1, TransitionKind::External)?;
assert_eq!(off.exit(), &[5, 7, 3, 6, 2]);

// A saved configuration is checked when it is restored; a source that is
// not active is refused.
assert_eq!(radio.configuration(&[5]), Err(Refusal::InvalidConfiguration));
let saved = radio.configuration(&[8, 4])?;
assert_eq!(radio.resolve(&saved, 1, 2, TransitionKind::External), Err(Refusal::Inactive));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## History: entering a state again where you left it

A compound state marked with history restores what it last held instead of
its initial child. Shallow restores the child; deep restores the whole
configuration underneath, and through a parallel state, every region.

```rust
use harel_machines::{Chart, History, LastActive, State, TransitionKind};

// 0 Download
// |- 1 Paused
// `- 2 Running (deep history)
//    `- 3 Transfer -> 4 Headers | 5 Body
let download = Chart::<6>::new(&[
    State::compound(None, 1),
    State::atomic(Some(0)),
    State::compound(Some(0), 3).with_history(History::Deep),
    State::compound(Some(2), 4),
    State::atomic(Some(3)),
    State::atomic(Some(3)),
])?;

let mut remembered = LastActive::new();
let start = download.enter(0)?;
let running = download.resolve(start.configuration(), 1, 2, TransitionKind::External)?;
remembered.record(&download, running.exit())?;
let body = download.resolve(running.configuration(), 4, 5, TransitionKind::External)?;
remembered.record(&download, body.exit())?;
let paused = download.resolve(body.configuration(), 2, 1, TransitionKind::External)?;
remembered.record(&download, paused.exit())?;

// Without history this would re-enter at Headers.
let resumed = download.resolve_with_history(
    paused.configuration(),
    1,
    2,
    TransitionKind::External,
    &remembered,
)?;
assert_eq!(resumed.entry(), &[2, 3, 5]);
assert_eq!(resumed.configuration().as_slice(), &[5]);

// Saved and restored, it is the same history.
let saved: Vec<_> = remembered.remembered().collect();
assert_eq!(saved, [(0, 2), (2, 3), (3, 5)]);
assert_eq!(LastActive::restored(&download, &saved)?, remembered);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Recording is the caller's call, and a separate one: resolving is pure, so a
transition that a guard later refuses leaves no trace.

## Internal transitions stay inside

The two kinds only differ when the source contains the target or is the target.

```rust
use harel_machines::{Chart, State, TransitionKind};

let call = Chart::<5>::new(&[
    State::compound(None, 1),
    State::atomic(Some(0)),
    State::compound(Some(0), 3),
    State::atomic(Some(2)),
    State::atomic(Some(2)),
])?;
let talking = call.configuration(&[4])?;

// A self transition that exits nothing, for a tick that must not re-run entry.
let internal = call.resolve(&talking, 4, 4, TransitionKind::Internal)?;
assert!(internal.exit().is_empty());
assert!(internal.entry().is_empty());

let external = call.resolve(&talking, 4, 4, TransitionKind::External)?;
assert_eq!(external.exit(), &[4]);
assert_eq!(external.entry(), &[4]);
# Ok::<(), Box<dyn std::error::Error>>(())
```

An external transition whose source contains or is its target (a self
transition, or a transition into the source's own subtree) exits the source
and everything below it, then enters the source again. It reaches no further:
a parallel region beside the source keeps the leaves it held. The root is a
source like any other, so a self transition on it re-enters the whole chart.

## What this is not

No events, no guards, no callbacks, no transition execution, no persistence and
no DSL: `state_machines` owns all of that. No deadlines, so an `after`
transition hands this crate a target that `deadline_machines` already resolved.
No retry (`chrono_machines`), no supervision (`supervisor_machines`), no
behaviour trees (`behavior_machines`).

The tree algebra is allocation-free and reads no clock, so it runs the same on
bare metal as it does under a web server.

## License

MIT.
