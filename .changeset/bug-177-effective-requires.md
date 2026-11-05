---
default: minor
closes: [BUG-177]
---

A suppression type can declare `effective_requires`, the fields a waiver must carry before it takes effect, for
example `effective_requires: [Reason]`. A waiver missing one, or holding a blank, placeholder or pending value, no
longer suppresses its finding; the finding is reported again, and `waiver.matches-nothing` reports the waiver as
`waiver-ineffective`. Without the declaration nothing changes.
