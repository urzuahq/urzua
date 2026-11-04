---
Stable-Id: 01M6JRKP5VVTR1S1FT45N9A7CV
Rule: citation.path-exists
Scope: docs/rfc/RFC-25-realized-by-s-identity-is-attribution-not-attestation.md
Reason: "Revision-log rows are append-only history, and this record's row names a path as it was when the row was written (a changeset fragment since consumed by a release). Rewriting the row to today's path would falsify what happened, and the row cannot be removed. The record's non-history text is already correct. `citation.path-exists` reads the revision log because it sits under the References heading; the waiver expires so it is revisited when the rule can skip the revision log."
Expires: 2027-05-04
---
# WAIVER-12 — RFC-25 history rows name paths as they were

## What this covers

`citation.path-exists` reports a path named in RFC-25's revision log that is not in the working tree. Revision-log rows are append-only history, and this record's row names a path as it was when the row was written (a changeset fragment since consumed by a release). Rewriting the row to today's path would falsify what happened, and the row cannot be removed. The record's non-history text is already correct. `citation.path-exists` reads the revision log because it sits under the References heading; the waiver expires so it is revisited when the rule can skip the revision log.

## References

- `docs/rfc/RFC-25-realized-by-s-identity-is-attribution-not-attestation.md` — the record whose text is covered.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed with the own-repo rollout of `citation.path-exists` (`MILE-143`). | **substantive** |
