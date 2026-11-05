---
default: minor
closes: []
---

New opt-in rule `embodiment.requires-accepted-status`: a record whose `Status` is not an accepted status
cannot state an `Embodiment` of `Implemented` or `Verified`. Accepted statuses are the type's declared
`statuses.success`; a type that declares none can list them in the rule's `accepted_statuses` option, and a
type with neither is not examined. `Specified`, `Not started` and `Inactive` are not flagged, and the rule's
census says how many records it judged.
