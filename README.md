# HarelMachines 🧭

> **Statechart hierarchy for Ruby: states inside states.** The command vessel
> that knows which decks you are leaving, in order, and which you are boarding.

A flat finite state machine answers "given this state and this event, what is
the next state". It has no answer for "what happens to the five states I was
inside of". **AMS Harel** is that answer and nothing else: given a tree and a
source/target pair the caller has already chosen, it reports which states to
exit, in order, and which to enter, in order.

---

## 🚀 Launch Sequence

```bash
# Add to your ship's Gemfile
gem 'harel_machines'
```

Requires **Ruby 4.0+** and a Rust toolchain, or a platform gem.

---

## 🎯 Mission Brief

```ruby
require 'harel_machines'

chart = HarelMachines::Chart.build do |c|
  c.compound :call, parent: nil, initial: :idle
  c.atomic :idle, parent: :call
  c.compound :active, parent: :call, initial: :dialing
  c.atomic :dialing, parent: :active
  c.atomic :talking, parent: :active
end

start = chart.enter(:call)
start.entry                 # => [:call, :idle]
start.configuration         # => [:idle]

dial = chart.resolve(start.configuration, :idle, :active, :external)
dial.exit                   # => [:idle]
dial.entry                  # => [:active, :dialing]

# Hanging up from inside leaves everything it was inside of, innermost first.
talk = chart.resolve(dial.configuration, :dialing, :talking, :external)
hang_up = chart.resolve(talk.configuration, :talking, :idle, :external)
hang_up.exit                # => [:talking, :active]
hang_up.entry               # => [:idle]
```

---

## 🚫 Not This Ship's Duty

The `state_machines` gem owns flat machines: states, events, guards, callbacks,
transition execution, the DSL and persistence. This gem owns one thing,
resolving a transition inside a hierarchy, and holds no events, no guards, no
callbacks and no data. There are no deadlines either, so an `after`
transition arrives here as a target `deadline_machines` already resolved.

---

## 🔩 The Native Extension Is Required

There is no pure-Ruby fallback and no switch to turn one on. `require
'harel_machines'` loads the extension and fails if it is not there. The gem is
a binding to the `harel-machines` crate in `ext/harel_machines_native/core`,
which is `no_std` with zero dependencies and also runs on bare metal.

---

## 🗺️ Laying Out the Decks

A chart is declared once and is immutable. The declaration mirrors the core's
states: `atomic` (a leaf), `compound` (one child active at a time, entered at
`initial:`) and `parallel` (every child a region, all active at once). Every
state names its `parent:`, `nil` for the root. `compound` and `parallel` take
`history:` (`:none`, the default as in the core, `:shallow` or `:deep`). A
parent or an initial child may name a state the block declares later.

```ruby
chart = HarelMachines::Chart.build do |c|
  c.compound :power, parent: nil, initial: :off
  c.atomic :off, parent: :power
  c.compound :on, parent: :power, initial: :idle
  c.atomic :idle, parent: :on
  c.atomic :busy, parent: :on
end

chart.states                                  # => [:power, :off, :on, :idle, :busy]
chart.root                                    # => :power
chart.parent(:idle)                           # => :on
chart.depth(:idle)                            # => 2
chart.ancestor?(:on, :idle)                   # => true
chart.least_common_ancestor(:idle, :busy)     # => :on
```

The core checks the tree when it is built, and so every chart that exists is
one tree: exactly one root, no cycle, every compound state's initial child its
own, every parallel state with at least one region, no atomic state with
children or history. Anything else raises `RangeError` (`"not a chart: a
chart has exactly one root"`), and so does a name declared twice or never.

State names are Symbols here and `u16` indices inside, and the chart holds the
mapping. A name no state was declared with raises `RangeError`; anything but
a Symbol raises `TypeError` (a String included).

A chart is sized at build time from a fixed set of shapes,
`HarelMachines::SHAPES`, which is `[8, 32, 128, 512]`. More states than the
largest raises `RangeError`.

---

## 🔀 A Transition Is an Exit Set and an Entry Set

`resolve(configuration, source, target, kind)` takes the leaves that are
active now, which is what makes a parallel region's other leaves come along,
and the source and target the caller chose. It answers a frozen
`HarelMachines::Transition`, or `:inactive` when the source is not active in
the configuration (neither one of its leaves nor a state containing one).

```ruby
step = chart.resolve(%i[idle], :idle, :busy, :external)
step.exit              # => [:idle]
step.entry             # => [:busy]
step.configuration     # => [:busy]

chart.resolve(%i[idle], :off, :busy, :external)   # => :inactive
```

The configuration is checked against the chart first: leaves it cannot be
in (two children of one compound state, a region of a parallel state left
out, a state that is not a leaf, one named twice) raise `RangeError`.
`chart.configuration(leaves)` runs that check alone and answers the leaves in
declaration order.

Siblings never disturb the parent that holds them: nothing above the least
common ancestor of source and target is touched.

When the source contains the target, an `:internal` transition keeps the
source itself active; its active descendants may exit and re-enter (on an
atomic source targeting itself it exits and enters nothing). The two kinds
only differ when the source contains the target or is the target.

```ruby
chart.resolve(%i[idle], :idle, :idle, :internal).exit  # => []
chart.resolve(%i[idle], :idle, :idle, :external).exit  # => [:idle]
```

An external transition whose source contains or is its target exits the source
and everything below it, then enters the source again, and reaches no further:
a parallel region beside the source keeps the leaves it held. The root is a
source like any other, so a self transition on it re-enters the whole chart.

`enter(target)` enters from nothing: every state from the root down to the
target, then down to a leaf in every region it opens.

```ruby
chart.enter(:on).entry   # => [:power, :on, :idle]
```

---

## 🛰️ Parallel Regions Make the Configuration a Set

A state whose children are regions is in all of them at once, so what a chart
is in is a set of leaves rather than one leaf. Two departments of the same ship
are both on duty, and an order to one is not an order to the other.

```ruby
radio = HarelMachines::Chart.build do |c|
  c.compound :radio, parent: nil, initial: :off
  c.atomic :off, parent: :radio
  c.parallel :on, parent: :radio
  c.compound :volume, parent: :on, initial: :quiet
  c.atomic :quiet, parent: :volume
  c.atomic :loud, parent: :volume
  c.compound :link, parent: :on, initial: :online
  c.atomic :online, parent: :link
  c.atomic :offline, parent: :link
end

on = radio.resolve(%i[off], :off, :on, :external)
on.entry             # => [:on, :volume, :quiet, :link, :online]
on.configuration     # => [:quiet, :online]

# Turning the volume up says nothing about the link.
radio.resolve(on.configuration, :quiet, :loud, :external).configuration
# => [:loud, :online]
```

---

## 🕰️ History: Entering a State Again Where You Left It

A compound or parallel state declared with `history:` restores what it last
held instead of its initial child. `:shallow` restores the child, `:deep`
restores the whole configuration underneath, through every region.

```ruby
download = HarelMachines::Chart.build do |c|
  c.compound :download, parent: nil, initial: :paused
  c.atomic :paused, parent: :download
  c.compound :running, parent: :download, initial: :transfer, history: :deep
  c.compound :transfer, parent: :running, initial: :headers
  c.atomic :headers, parent: :transfer
  c.atomic :body, parent: :transfer
end

history = HarelMachines::History.new(download)
history.record(download.resolve(%i[body], :running, :paused, :external))   # => :recorded

resumed = download.resolve_with_history(%i[paused], :paused, :running, :external, history)
resumed.entry            # => [:running, :transfer, :body]
resumed.configuration    # => [:body]

history.remembered       # => [[:download, :running], [:running, :transfer], [:transfer, :body]]
history.last_child(:transfer)                                   # => :body
HarelMachines::History.restored(download, history.remembered)  # the same history
```

Recording is the caller's call and a separate one: resolving is pure, so a
transition a guard later refuses leaves no trace. `enter_with_history` reads
history too. A history belongs to the chart it was made for; using it with
another raises `RangeError`.

`History.restored(chart, pairs)` rebuilds a saved history and accepts exactly
what recorded transitions can leave: a state is exited with everything active
below it, so a remembered compound child remembers a child too, and a
compound state that remembers nothing has something remembered below at most
one of its children. Anything else raises `RangeError`. `record` answers
`:invalid_history`, and files nothing, for an exit set no transition of the
chart has.

---

## ⚖️ The Rules of the Bridge

- **Refusals are Symbols and never raise.** `:inactive` from `resolve` and
  `:invalid_history` from `record`; `HarelMachines::REFUSALS` lists the
  core's four. Malformed input raises: `TypeError` for the wrong type,
  `RangeError` for a value the chart does not hold (an unknown name, a
  configuration or history it cannot be in, a tree the core refuses),
  `ArgumentError` only for arity and keywords, `LocalJumpError` for
  `Chart.build` without a block.
- **Values are answers, and the binding is the authority.** `Transition` is a
  frozen `Data` whose Arrays are frozen; like the core's, only a chart builds
  one, so `new`, `[]` and `with` are gone. Every class is final: subclassing
  raises `TypeError` and `allocate` is gone. That stops accidents, not
  reflection (`send(:new)`, `Marshal.load`), so what the binding reads back,
  the Transition given to `History#record`, must be exactly a `Transition`
  (`TypeError` otherwise, a subclass included), its fields are read from the
  struct and never through a reader, and its exit set is checked against the
  chart: an unknown name raises `RangeError`, and an exit set no transition
  of the chart has is `:invalid_history`.
- **Marshal.** A `Transition` round-trips through `Marshal` as itself and
  records like the original. `Chart` and `History` are native state:
  `Marshal.dump`, `dup` and `clone` raise `TypeError`. Save a history with
  `remembered` and rebuild it with `History.restored`.
- **Frozen means frozen.** A frozen `History` raises `FrozenError` on
  `record` before its argument is read, and again just before it changes. A
  `Chart` never changes.
- **Classes are fixed at load.** `Chart`, `Chart::Declaration`, `History` and
  `Transition` are resolved and pinned when the extension loads: building
  one runs no Ruby, redefining a constant later changes nothing the gem
  builds or accepts, and `GC.compact` moves nothing the extension holds.

---

## 📡 Credits

**AMS Harel**, NCC-3187, *Harel*-class command vessel.

Named for David Harel's *Statecharts: A Visual Formalism for Complex Systems*
(1987), which gave a flat state machine depth, parallel regions and history,
and so gave the rest of us something to resolve.

---

## License

MIT.
