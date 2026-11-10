---
default: minor
closes: []
---

`diff.decider-required` now treats deleting a record that held a gated status (for example an Accepted ADR) as a
gated move that needs a decider, and the post-command check reports such a deletion; a rename is not a deletion.
Gate statuses are compared case-insensitively, so `accepted` or `ACCEPTED` can no longer slip past a gate on
`Accepted`.
