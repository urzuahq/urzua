---
default: minor
closes: []
---

`urzua check` now reports an unknown key in the `harness` configuration (a misspelled guardrail
setting such as `enabeld`, or an unknown top-level `harness` key) as a warning notice with a structured
`detail` holding the `key`, its `nearest` valid key and the `valid` keys, instead of staying silent. It
never moves the exit code. A freshly scaffolded milestone no longer fails `status.done-items-shape`
before it is filled, and `urzua --help` now carries the one-line description the README and `RFC-27`
state.
