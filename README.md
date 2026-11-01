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
  c.compound :call, initial: :idle
  c.state :idle, parent: :call
  c.compound :active, parent: :call, initial: :dialing
  c.state :dialing, parent: :active
  c.state :talking, parent: :active
end

start = chart.enter(:call)
start.entry                 # => [:call, :idle]
start.configuration         # => [:idle]

dial = chart.resolve(start.configuration, from: :idle, to: :active)
dial.exit                   # => [:idle]
dial.entry                  # => [:active, :dialing]

# Hanging up from inside leaves everything it was inside of, innermost first.
talk = chart.resolve(dial.configuration, from: :dialing, to: :talking)
hang_up = chart.resolve(talk.configuration, from: :talking, to: :idle)
hang_up.exit                # => [:talking, :active]
hang_up.entry               # => [:idle]
```

---

## 🚫 Not This Ship's Duty

The `state_machines` gem owns flat machines: states, events, guards, callbacks,
transition execution, the DSL and persistence. This gem owns one thing,
resolving a transition inside a hierarchy, and holds no events, no guards, no
callbacks, no DSL and no data. There are no deadlines either, so an `after`
transition arrives here as a target `deadline_machines` already resolved.

---

## 🔩 The Native Extension Is Required

There is no pure-Ruby fallback and no switch to turn one on. `require
'harel_machines'` loads the extension and fails if it is not there. The gem is
a binding to the `harel-machines` crate in `ext/harel_machines_native/core`,
which is `no_std` with zero dependencies and also runs on bare metal. It needs
Ruby 4.0 and a Rust toolchain, or a platform gem.

---

## 🗺️ Laying Out the Decks

A chart is declared once and is immutable. A parent or an initial child may
name a state the block declares later.

```ruby
chart = HarelMachines::Chart.build do |c|
  c.state :off
  c.compound :on, initial: :idle
  c.state :idle, parent: :on
  c.state :busy, parent: :on
end

chart.states                                  # => [:off, :on, :idle, :busy]
chart.depth(:idle)                            # => 1
chart.ancestor?(:on, :idle)                   # => true
chart.least_common_ancestor(:idle, :busy)     # => :on
```

State names are symbols here and `u16` indices inside, and the chart holds the
mapping. A name no state was declared with raises `ArgumentError`.

`:off` and `:on` above are both roots, and two roots share no ancestor, so a
transition between them has nothing to bound it and raises
`HarelMachines::NoCommonAncestor`. Give a chart a single root unless its trees
really are disjoint.

A chart is sized at build time from a fixed set of shapes,
`HarelMachines::SHAPES`, which is `[8, 32, 128, 512]`. More states than the
largest raises `ArgumentError`.

---

## 🔀 A Transition Is an Exit Set and an Entry Set

`resolve` takes the leaves that are active now, which is what makes a parallel
region's other leaves come along, and the source and target the caller chose.
It returns a frozen `HarelMachines::Transition`.

```ruby
step = chart.resolve(%i[idle], from: :idle, to: :busy)
step.exit              # => [:idle]
step.entry             # => [:busy]
step.configuration     # => [:busy]
```

Siblings never disturb the parent that holds them: nothing at or above the
least common ancestor of source and target is touched.

`kind: :internal` keeps the source active, and everything between it and the
target with it. The two kinds only differ when the source contains the target
or is the target.

```ruby
chart.resolve(%i[idle], from: :idle, to: :idle, kind: :internal).exit  # => []
chart.resolve(%i[idle], from: :idle, to: :idle).exit                   # => [:idle]
```

An external transition whose source contains or is its target exits the source
and everything below it, then enters the source again, and reaches no further:
a parallel region beside the source keeps the leaves it held. A root is a
source like any other, so a self transition on one re-enters the whole chart.

---

## 🛰️ Parallel Regions Make the Configuration a Set

A state whose children are regions is in all of them at once, so what a chart
is in is a set of leaves rather than one leaf. Two departments of the same ship
are both on duty, and an order to one is not an order to the other.

```ruby
radio = HarelMachines::Chart.build do |c|
  c.compound :radio, initial: :off
  c.state :off, parent: :radio
  c.parallel :on, parent: :radio
  c.compound :volume, parent: :on, initial: :quiet
  c.state :quiet, parent: :volume
  c.state :loud, parent: :volume
  c.compound :link, parent: :on, initial: :online
  c.state :online, parent: :link
  c.state :offline, parent: :link
end

on = radio.resolve(%i[off], from: :off, to: :on)
on.entry             # => [:on, :volume, :quiet, :link, :online]
on.configuration     # => [:quiet, :online]

# Turning the volume up says nothing about the link.
radio.resolve(on.configuration, from: :quiet, to: :loud).configuration
# => [:loud, :online]
```

---

## 🕰️ History: Entering a State Again Where You Left It

A composite state declared with `history:` restores what it last held instead
of its initial child. `:shallow` restores the child, `:deep` restores the whole
configuration underneath.

```ruby
download = HarelMachines::Chart.build do |c|
  c.compound :download, initial: :paused
  c.state :paused, parent: :download
  c.compound :running, parent: :download, initial: :transfer, history: :deep
  c.compound :transfer, parent: :running, initial: :headers
  c.state :headers, parent: :transfer
  c.state :body, parent: :transfer
end

history = HarelMachines::History.new(download)
history.record(download.resolve(%i[body], from: :running, to: :paused))

resumed = download.resolve(%i[paused], from: :paused, to: :running, history: history)
resumed.entry            # => [:running, :transfer, :body]
resumed.configuration    # => [:body]
```

Recording is the caller's call and a separate one: resolving is pure, so a
transition a guard later refuses leaves no trace. `enter` reads history too.

---

## 📡 Credits

**AMS Harel**, NCC-3187, *Harel*-class command vessel.

Named for David Harel's *Statecharts: A Visual Formalism for Complex Systems*
(1987), which gave a flat state machine depth, parallel regions and history,
and so gave the rest of us something to resolve.

---

## License

MIT.
