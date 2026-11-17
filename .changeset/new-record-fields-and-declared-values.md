---
default: minor
closes: []
---

`urzua new` changes what it writes: a field it has no value for is no longer written as `null`. It carries the type's template text for that field (for example `Deciders: name(s)`), or `—` when the type requires the field and the template has no text for it, or is left out when the field is optional; an unset optional pointer field is therefore left out and trips no pointer rule. `new`'s report gains `to_fill`, listing each such field with `placeholder` or `required-empty`, and its next-action reason names them. A value a type declares for a field is no longer reported as a placeholder or pending in that field: a status named `Todo` in `statuses.values` passes `field.quality` and `field.pending` in the status field, while the same word in another field is still reported. `urzua doctor` names such a value at a new `info` check status, `declared-value-vocabulary`, which does not change the report's status.
