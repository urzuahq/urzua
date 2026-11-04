---
Stable-Id: 01M6JN5765DA5DCEV4GYW3C5S8
Rule: relation.pair-reciprocity
Scope: docs/rfc/RFC-25-realized-by-s-identity-is-attribution-not-attestation.md
Reason: RFC-25 is a Draft that amends RFC-2. RFC-2 is an accepted record and its Amended-by cannot assert an amendment no one has accepted, so the provisional warning is expected until RFC-25 is decided. It becomes an error when RFC-25 is accepted without RFC-2 naming it back.
Expires: 2027-02-04
---
# 1 — RFC-25 amends RFC-2 while it is a Draft

## What this covers

The provisional `relation.pair-reciprocity` warning on RFC-25's `Amends: RFC-2` claim (MILE-154). The
claim is deliberate and the claimant is a Draft, so the finding is correct but is not yet a defect to fix.
The waiver stops blocking `fail_on: warning` and expires so the exception cannot outlive the draft.

## References

- `MILE-154` -- the provisional claim this repository now exercises.
- `MILE-127` -- the decision that a claim from a record not yet accepted is a warning.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** `MILE-154` declares a provisional split on the amendment pair; RFC-25 (Draft) amends RFC-2, which is the case it exists for. | **substantive** |
