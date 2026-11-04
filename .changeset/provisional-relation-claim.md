---
default: minor
---

Add `provisional` to a `relation_pairs` entry: `provisional: {statuses: [Draft, Proposed], level: warn}`. `relation.pair-reciprocity` then reports a claim from a record at one of those statuses that its peer does not echo at the provisional level (`warn` by default), while a claim from any other status keeps the rule's own level, so it becomes an error once the claimant is accepted. Off unless written; a status that no record type declares is refused at load.
