---
default: minor
---

A record's `Status` can follow any evidence field its type declares. `statuses.delivery` already held a
type's `Status` to its `Realized-by` locators; it now applies to any record type, and takes an optional
`back_pointer_evidence` key (default `true`) so a type can say that only its declared evidence field
counts, not a source back-pointer that cites the record. `unevidenced_tiers` may be an empty list for a
type whose done records legitimately have no evidence. A record that is deliberately open with evidence
is waived with a `waiver` record scoped to its path, with a reason and an expiry.
