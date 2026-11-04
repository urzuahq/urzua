---
default: minor
---

Add `relation_pairs` to `.urzua/config.yaml`: declare a pair of inverse relation fields (a forward
field and its inverse), whether the two ends may be different record types, and whether reciprocity
is required. Two new opt-in rules use it. `relation.pair-reciprocity` reports a claim on one side
that the other side does not echo, in either direction and across record types.
`relation.cross-type-disallowed` reports a claim to another record type in a pair that does not allow
it. Enabling either rule without a declared pair fails the config load. Existing configs and
`relation.supersession-reciprocity` are unchanged.
