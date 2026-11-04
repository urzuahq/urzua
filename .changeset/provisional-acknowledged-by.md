---
default: minor
---

A relation pair's `provisional` declaration takes an optional `acknowledged_by` field. A provisional claim
(from a claimant at one of its statuses) whose peer names the claimant in that field is settled by the peer's
own declaration and is not reported by `relation.pair-reciprocity`; a claim from any other status still needs
the inverse relation.
