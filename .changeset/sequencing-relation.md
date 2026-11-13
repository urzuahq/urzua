---
default: minor
closes: [MILE-27]
---

Add `acyclic: true` to a `relation_pairs` entry to declare it a sequencing relation, for example
`Depends-on` with its inverse `Blocks`. Two new opt-in rules read it. `sequence.cycle` reports every
record on a cycle with the cycle's path, including a record that depends on itself and a cycle across
record types. `sequence.unresolved-blocker` reports a value in either field that names no record,
whether a reference that resolves to nothing or prose. Enabling either rule with no acyclic pair
declared fails the config load. Existing configs are unchanged.
