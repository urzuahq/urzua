---
default: patch
closes: []
---

A record type's `statuses` declaration accepts an optional `implementable` list: the statuses at
which implementation of the record is permitted, which must be among its `values`.
`embodiment.back-pointer-decision-accepted` now judges a back-pointer against that list instead of
the type's terminal `success` statuses, falling back to the rule's `accepted_statuses` option and
reporting the status as unjudgeable when neither is declared.
