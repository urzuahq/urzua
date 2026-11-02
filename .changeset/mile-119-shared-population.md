---
default: patch
---

`urzua check` now reports a record of a `header_shape: none` type as out of scope, rather than as
unreadable or unexamined, in every rule that reads headers. Twelve rules previously counted such
records as `unreadable` in their population (`header.field-set-consistency`,
`header.pointer-field-clean`, `field.untrimmed-value`, `header.field-case-mismatch`,
`pointer.resolution`, `pointer.target-status`, `narrative-field.stale`, the three `embodiment.*`
rules, `relation.supersession-reciprocity` and `relation.target-status-undeclared`), implying a
malformed corpus where the configuration declares no header at all, and `header.layout-consistency`
counted them as `absent` (eligible but not examined) when the type declared a `header_layout`.
