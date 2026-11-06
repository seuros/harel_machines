# Changelog

## [0.1.3](https://github.com/seuros/harel_machines/compare/harel_machines/v0.1.2...harel_machines/v0.1.3) (2026-11-06)


### Bug Fixes

* drop descend in favor of descend_along ([0db2955](https://github.com/seuros/harel_machines/commit/0db29552c6a65812741c302e28be91e23b1efbf0))
* keep test modules out of the packaged crate ([b0c1029](https://github.com/seuros/harel_machines/commit/b0c1029211aace799829695c5091d2663d8aba66))

## [0.1.2](https://github.com/seuros/harel_machines/compare/harel_machines/v0.1.1...harel_machines/v0.1.2) (2026-11-01)


### Bug Fixes

* a republished platform gem is skipped, not fatal ([ad72166](https://github.com/seuros/harel_machines/commit/ad7216674d6401fea3984aa94624ab262057ce6b))
* enable the rb-sys compiled fallback so new Rubies can build ([2d10131](https://github.com/seuros/harel_machines/commit/2d10131913ebe8d80545a800f698829df506e5fd))

## [0.1.1](https://github.com/seuros/harel_machines/compare/harel_machines-v0.1.0...harel_machines/v0.1.1) (2026-11-01)


### Features

* pass a rubygems OTP through the environment ([d610ea6](https://github.com/seuros/harel_machines/commit/d610ea630c8af48342b86a4000052afdecd4e633))
* Ruby gem and magnus FFI for chart resolution ([8a6da13](https://github.com/seuros/harel_machines/commit/8a6da130d0181db588f7906fd511c350ef32c85d))
* statechart hierarchy resolution ([5f37158](https://github.com/seuros/harel_machines/commit/5f37158accd85760fa148b003937c82258a946f9))


### Bug Fixes

* a cross-region transition left a parallel region empty ([80ffdc3](https://github.com/seuros/harel_machines/commit/80ffdc3e609cf8f835aaef7d835c48b11a2e4617))
* a source install can build, and a failed gem push fails the release ([95a6eab](https://github.com/seuros/harel_machines/commit/95a6eab5231bb5069c7b4ef826cf35d3ba9ad7c4))
* an external self-transition no longer resets a sibling region ([3e487ce](https://github.com/seuros/harel_machines/commit/3e487cef4dd02cb0a3c26c22a9cc8614b03969bc))
* collect the built gems whatever depth the download puts them at ([0281ea9](https://github.com/seuros/harel_machines/commit/0281ea9743a63cd907451a72b00142f1d15977c6))
* entering through a parallel ancestor activates every region ([5296c70](https://github.com/seuros/harel_machines/commit/5296c702d607985e3de5bb6cfffc9a7f22022cb0))
* entry order, duplicate names, and children under an atomic state ([b7e68bc](https://github.com/seuros/harel_machines/commit/b7e68bcf4d1e8ceed757fc192b7b39f6c9ac9917))
* refuse a compound state whose initial is not its child ([3120e48](https://github.com/seuros/harel_machines/commit/3120e486d5a3e0bc4ff345ef6adfe35ea62f1872))
* refuse a history recorded on another chart ([eb27f4e](https://github.com/seuros/harel_machines/commit/eb27f4e954aa88272a43a476ba8f3abbb0ee20e4))
* ship the core's test files so the packaged crate builds ([62d1e3f](https://github.com/seuros/harel_machines/commit/62d1e3fa8c1c530a145cdc43e211aa6dd139ecdd))
* the release tag carries the gem name, and the gate uses the aggregate output ([eb17b76](https://github.com/seuros/harel_machines/commit/eb17b76fe7204935c1b73633f4d603742e899c4e))

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
