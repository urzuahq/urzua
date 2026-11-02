---
default: patch
---

`urzua check` now reports a record of a `header_shape: none` type as out of scope, rather than as
unreadable, in every rule that reads headers. Thirteen rules previously counted such records as
`unreadable` in their population (`header.layout-consistency`, `header.field-set-consistency`,
`header.pointer-field-clean`, `field.untrimmed-value`, `header.field-case-mismatch`,
`pointer.resolution`, `pointer.target-status`, `narrative-field.stale`, the three `embodiment.*`
rules, `relation.supersession-reciprocity` and `relation.target-status-undeclared`), implying a
malformed corpus where the configuration declares no header at all.
