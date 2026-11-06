# Changelog

## [0.2.1](https://github.com/seuros/harel_machines/compare/harel-machines-v0.2.0...harel-machines-v0.2.1) (2026-11-06)


### Bug Fixes

* drop descend in favor of descend_along ([0db2955](https://github.com/seuros/harel_machines/commit/0db29552c6a65812741c302e28be91e23b1efbf0))
* keep test modules out of the packaged crate ([b0c1029](https://github.com/seuros/harel_machines/commit/b0c1029211aace799829695c5091d2663d8aba66))

## [0.2.0](https://github.com/seuros/harel_machines/compare/harel-machines-v0.1.0...harel-machines-v0.2.0) (2026-11-01)


### Features

* Ruby gem and magnus FFI for chart resolution ([8a6da13](https://github.com/seuros/harel_machines/commit/8a6da130d0181db588f7906fd511c350ef32c85d))
* statechart hierarchy resolution ([5f37158](https://github.com/seuros/harel_machines/commit/5f37158accd85760fa148b003937c82258a946f9))


### Bug Fixes

* a cross-region transition left a parallel region empty ([80ffdc3](https://github.com/seuros/harel_machines/commit/80ffdc3e609cf8f835aaef7d835c48b11a2e4617))
* an external self-transition no longer resets a sibling region ([3e487ce](https://github.com/seuros/harel_machines/commit/3e487cef4dd02cb0a3c26c22a9cc8614b03969bc))
* entering through a parallel ancestor activates every region ([5296c70](https://github.com/seuros/harel_machines/commit/5296c702d607985e3de5bb6cfffc9a7f22022cb0))
* entry order, duplicate names, and children under an atomic state ([b7e68bc](https://github.com/seuros/harel_machines/commit/b7e68bcf4d1e8ceed757fc192b7b39f6c9ac9917))
* ship the core's test files so the packaged crate builds ([62d1e3f](https://github.com/seuros/harel_machines/commit/62d1e3fa8c1c530a145cdc43e211aa6dd139ecdd))
