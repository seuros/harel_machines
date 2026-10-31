# harel-machines

Statechart hierarchy: states inside states. `no_std`, zero dependencies.

David Harel's 1987 statecharts added to the flat finite state machine the one
thing it never had: a state can contain other states. A flat machine answers
"given this state and this event, what is the next state". It has no answer for
"what happens to the five states I was inside of".

This crate is that answer and nothing else. It holds no events, no guards, no
callbacks, no DSL and no data. The caller owns the machine — `state_machines`
already owns the flat part — and asks here which states to leave and which to
join.

## A transition is an exit set and an entry set

Give it a tree and a transition. It gives back the states to exit, innermost
first, and the states to enter, outermost first, bounded by the least common
ancestor of source and target.

```rust
use harel_machines::{Chart, Configuration, State, TransitionKind};

// 0 Call
// |- 1 Idle
// `- 2 Active
//    |- 3 Dialing
//    `- 4 Talking
const CALL: Chart<5> = Chart::new([
    State::compound(None, 1),
    State::atomic(Some(0)),
    State::compound(Some(0), 3),
    State::atomic(Some(2)),
    State::atomic(Some(2)),
]);

// Entering a composite state means entering one of its children, so entry
// keeps descending until it reaches a leaf.
let start = CALL.enter(0)?;
assert_eq!(start.entry(), &[0, 1]);
assert_eq!(start.configuration().as_slice(), &[1]);

let dial = CALL.resolve(start.configuration(), 1, 2, TransitionKind::External)?;
assert_eq!(dial.exit(), &[1]);
assert_eq!(dial.entry(), &[2, 3]);

// Siblings never disturb the parent that holds them.
let talk = CALL.resolve(dial.configuration(), 3, 4, TransitionKind::External)?;
assert_eq!(talk.exit(), &[3]);
assert_eq!(talk.entry(), &[4]);

// Hanging up from inside leaves everything it was inside of, innermost first.
let hang_up = CALL.resolve(talk.configuration(), 4, 1, TransitionKind::External)?;
assert_eq!(hang_up.exit(), &[4, 2]);
assert_eq!(hang_up.entry(), &[1]);
assert_eq!(hang_up.configuration().as_slice(), &[1]);
# Ok::<(), harel_machines::Error>(())
```

## Parallel regions make the configuration a set

A state whose children are regions is in all of them at once, so what a chart
is "in" is a set of leaves, not one leaf.

```rust
use harel_machines::{Chart, State, TransitionKind};

// 0 Radio
// |- 1 Off
// `- 2 On (parallel)
//    |- 3 Volume  -> 4 Muted | 5 Loud
//    `- 6 Link    -> 7 Online | 8 Offline
const RADIO: Chart<9> = Chart::new([
    State::compound(None, 1),
    State::atomic(Some(0)),
    State::parallel(Some(0)),
    State::compound(Some(2), 4),
    State::atomic(Some(3)),
    State::atomic(Some(3)),
    State::compound(Some(2), 7),
    State::atomic(Some(6)),
    State::atomic(Some(6)),
]);

let on = RADIO.resolve(RADIO.enter(0)?.configuration(), 1, 2, TransitionKind::External)?;
assert_eq!(on.entry(), &[2, 3, 4, 6, 7]);
assert_eq!(on.configuration().as_slice(), &[4, 7]);

// Turning the volume up says nothing about the link.
let loud = RADIO.resolve(on.configuration(), 4, 5, TransitionKind::External)?;
assert_eq!(loud.configuration().as_slice(), &[5, 7]);

// Turning the radio off leaves both regions, children before parents.
let off = RADIO.resolve(loud.configuration(), 2, 1, TransitionKind::External)?;
assert_eq!(off.exit(), &[5, 7, 3, 6, 2]);
# Ok::<(), harel_machines::Error>(())
```

## History: entering a state again where you left it

A composite state marked with history restores what it last held instead of its
initial child. Shallow restores the child; deep restores the whole
configuration underneath.

```rust
use harel_machines::{Chart, History, LastActive, State, TransitionKind};

// 0 Download
// |- 1 Paused
// `- 2 Running (deep history)
//    `- 3 Transfer -> 4 Headers | 5 Body
const DOWNLOAD: Chart<6> = Chart::new([
    State::compound(None, 1),
    State::atomic(Some(0)),
    State::compound(Some(0), 3).with_history(History::Deep),
    State::compound(Some(2), 4),
    State::atomic(Some(3)),
    State::atomic(Some(3)),
]);

let mut remembered = LastActive::new();

let running = DOWNLOAD.resolve(
    DOWNLOAD.enter(0)?.configuration(),
    1,
    2,
    TransitionKind::External,
)?;
remembered.record(&DOWNLOAD, &running);

let body = DOWNLOAD.resolve(running.configuration(), 4, 5, TransitionKind::External)?;
remembered.record(&DOWNLOAD, &body);

let paused = DOWNLOAD.resolve(body.configuration(), 2, 1, TransitionKind::External)?;
remembered.record(&DOWNLOAD, &paused);

// Without history this would re-enter at Headers.
let resumed = DOWNLOAD.resolve_with_history(
    paused.configuration(),
    1,
    2,
    TransitionKind::External,
    &remembered,
)?;
assert_eq!(resumed.entry(), &[2, 3, 5]);
assert_eq!(resumed.configuration().as_slice(), &[5]);
# Ok::<(), harel_machines::Error>(())
```

Recording is the caller's call, and a separate one: resolving is pure, so a
transition that a guard later refuses leaves no trace.

## Internal transitions stay inside

The two kinds only differ when the source contains the target or is the target.

```rust
use harel_machines::{Chart, Configuration, State, TransitionKind};

const CALL: Chart<5> = Chart::new([
    State::compound(None, 1),
    State::atomic(Some(0)),
    State::compound(Some(0), 3),
    State::atomic(Some(2)),
    State::atomic(Some(2)),
]);

let mut talking = Configuration::new();
talking.insert(4);

// A self transition that exits nothing, for a tick that must not re-run entry.
let internal = CALL.resolve(&talking, 4, 4, TransitionKind::Internal)?;
assert!(internal.exit().is_empty());
assert!(internal.entry().is_empty());

let external = CALL.resolve(&talking, 4, 4, TransitionKind::External)?;
assert_eq!(external.exit(), &[4]);
assert_eq!(external.entry(), &[4]);
# Ok::<(), harel_machines::Error>(())
```

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
