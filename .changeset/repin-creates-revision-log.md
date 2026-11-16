---
default: minor
closes: []
---

`urzua fix --rebaseline --apply` and `urzua fix --pin --apply` create the revision log on a record that has none, in the shape the type's template or `revision_log` declaration gives (the narrative header from the new optional `revision_log.narrative_column`, default `Change`), placed where the template orders its log, with the evidence row as its first row. Listings mark such a record `creates_revision_log: true`. Refused in both the listing and the apply: a record with no heading below its header (`next_action` appends one), one ending inside an open code fence, and one whose committed copy had a log, which must be restored rather than replaced.
