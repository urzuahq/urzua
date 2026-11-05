---
default: minor
closes: []
---

`field.pending` takes the optional `pending_until_required` option. Declared `true`, a pending value in a
field the type's `statuses.require` ties to certain statuses is not reported until the record reaches one
of them, so a placeholder such as "not yet written" on an open bug is no longer a warning while the same
placeholder on a fixed bug still is. A deferred slot stays counted as eligible and unexamined, a field no
`require` entry names is judged as before, and the option is off unless declared.
