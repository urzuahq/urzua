---
default: minor
closes: [MILE-203]
---

A rule can declare `scope: changed` so it judges only the records a `--base` diff adds or changes, and none on a plain `check`. It is meant for advisory rules, whose findings are useful at the moment someone touches a record and nowhere else: `check --stdin-path` treats the proposed record as changed, a record that merely depends on a changed file is not nudged, and the report says what was left unjudged. A rule's population gains a `deferred_by_scope` counter, present only when it is non-zero, and the report carries one `info` notice for each such rule naming how many slots were not judged and why; `config.scope-matches-nothing` does not warn about them, and `waiver.matches-nothing` leaves a waiver on such a rule alone. `scope` takes one value, `changed`; any other value, or a rule that does not take the option, fails to load. `urzua rules` now prints each rule's `options` and whether it can take `scope` (`judges`). `embodiment.locator-on-back-pointer-path` takes the option.
