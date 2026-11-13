---
default: minor
closes: []
---

`urzua explain --rule <rule-id>` prints what one rule is, as JSON: its scope, the level this
repository sets for it, its severity floor and whether a waiver can clear it, its options with their
current values, the suggested actions its findings offer, its population unit and the spec that
documents it. A bare argument to `explain` is still always a path, so a rule id and a path are never
confused. An unknown rule id is an error that suggests the nearest valid id. `urzua rules` also prints
each rule's `scope`, `population_unit` and `spec`.
