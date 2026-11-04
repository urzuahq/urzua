---
Stable-Id: 01M6JRKVP7RQDC8F7Y5W8FZMK1
Rule: citation.resolution
Scope: docs/adr/ADR-14-an-append-only-revision-log-not-edit-in-place.md
Reason: "Accepted decision text (an amendment) records that a spec number was minted and then retracted. The id names a record that existed under that numbering and no longer does; rewording would falsify the history the amendment exists to state. Reviewed finding by finding; expires so the exception is revisited."
Expires: 2027-11-04
---
# WAIVER-22 — ADR-14 cites ids that are history or illustration

## What this covers

`citation.resolution` reports ids cited in ADR-14's prose that resolve to no record. Accepted decision text (an amendment) records that a spec number was minted and then retracted. The id names a record that existed under that numbering and no longer does; rewording would falsify the history the amendment exists to state.

## References

- `docs/adr/ADR-14-an-append-only-revision-log-not-edit-in-place.md` — the record whose text is covered.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed with the own-repo rollout of `citation.resolution` (`MILE-156`). | **substantive** |
