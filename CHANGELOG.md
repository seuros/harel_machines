# Changelog

## 0.1.0

- Initial release.
- `HarelMachines::Chart`: an immutable state tree, declared once through
  `Chart.build` with `state`, `compound` and `parallel`, answering `states`,
  `depth`, `ancestor?` and `least_common_ancestor`.
- `HarelMachines::Chart#enter` and `#resolve`: the exit set innermost first,
  the entry set outermost first, and the configuration of active leaves that
  results, bounded by the least common ancestor of source and target. Both
  return a frozen `HarelMachines::Transition`.
- Parallel regions, so a configuration is a set of leaves rather than one leaf,
  and `kind: :internal` for a transition that does not leave its source.
- `HarelMachines::History`: shallow and deep history, recorded by the caller
  and read by `enter` and `resolve`, so a transition a guard refuses leaves no
  trace.
- The core's error variants raise as `HarelMachines::NoCommonAncestor`,
  `EmptyParallel` and `NotATree`, all under `HarelMachines::Error`.
- State names are symbols at the boundary and `u16` indices inside; a chart
  holds the mapping and raises `ArgumentError` for a name it does not hold.
- A chart is monomorphised at one of `HarelMachines::SHAPES`, `[8, 32, 128,
  512]` states; more than the largest raises `ArgumentError`.
- `harel-machines` core crate: the tree algebra in `no_std` Rust with zero
  dependencies, allocation-free and reading no clock. Builds and lints clean on
  bare metal.
- RBS signatures for the whole public API.
- No events, guards, callbacks, transition DSL, completion transitions or
  persistence: `state_machines` owns those. No deadlines, retry, supervision or
  behaviour trees either.
