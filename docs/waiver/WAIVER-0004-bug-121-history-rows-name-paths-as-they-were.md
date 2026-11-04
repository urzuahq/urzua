---
Stable-Id: 01M6JRKJ315ABCQVTSPC0BJATT
Rule: citation.path-exists
Scope: docs/bugs/BUG-121-check-s-path-scope-filter-still-drops-a-claim-status-agreement-finding-whose-claim-file-falls-outside-the-requested-scope.md
Reason: "Revision-log rows are append-only history, and this record's row names a path as it was when the row was written (a directory of a hypothetical fixture). Rewriting the row to today's path would falsify what happened, and the row cannot be removed. The record's non-history text is already correct. `citation.path-exists` reads the revision log because it sits under the References heading; the waiver expires so it is revisited when the rule can skip the revision log."
Expires: 2027-05-04
---
# WAIVER-4 — BUG-121 history rows name paths as they were

## What this covers

`citation.path-exists` reports a path named in BUG-121's revision log that is not in the working tree. Revision-log rows are append-only history, and this record's row names a path as it was when the row was written (a directory of a hypothetical fixture). Rewriting the row to today's path would falsify what happened, and the row cannot be removed. The record's non-history text is already correct. `citation.path-exists` reads the revision log because it sits under the References heading; the waiver expires so it is revisited when the rule can skip the revision log.

## References

- `docs/bugs/BUG-121-check-s-path-scope-filter-still-drops-a-claim-status-agreement-finding-whose-claim-file-falls-outside-the-requested-scope.md` — the record whose text is covered.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed with the own-repo rollout of `citation.path-exists` (`MILE-143`). | **substantive** |
