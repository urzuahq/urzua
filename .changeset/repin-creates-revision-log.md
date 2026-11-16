---
default: minor
closes: []
---

`urzua fix --rebaseline --apply` and `urzua fix --pin --apply` create the revision log on a record that has none, in the shape the type's template or `revision_log` declaration gives, with the evidence row as its first row, so `diff.pin-changed-unattested` reads a tool-written row instead of a hand-written log. The listing marks such a record `creates_revision_log: true`; a record with no heading below its header is refused with a `next_action` that adds one.
