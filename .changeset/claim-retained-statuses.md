---
default: patch
closes: []
---

A record type's `statuses` declaration accepts an optional `claim_retained` list: statuses reached
only after acceptance, such as `Superseded`, that may still carry an `Implemented` or `Verified`
embodiment claim. `embodiment.requires-accepted-status` allows the claim at those statuses as well
as at the type's `success` statuses.
