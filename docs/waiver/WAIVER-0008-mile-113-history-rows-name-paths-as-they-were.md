---
Stable-Id: 01M6JRKM5HXXHSHBMXSH2E3A72
Rule: citation.path-exists
Scope: docs/milestones/MILE-113-an-opt-in-rule-flagging-two-keys-in-one-record-differing-only-by-case-composable-alongside-exact-match-duplicate-detection.md
Reason: "Revision-log rows are append-only history, and this record's row names a path as it was when the row was written (source files by crate-relative paths). Rewriting the row to today's path would falsify what happened, and the row cannot be removed. The record's non-history text is already correct. `citation.path-exists` reads the revision log because it sits under the References heading; the waiver expires so it is revisited when the rule can skip the revision log."
Expires: 2027-05-04
---
# WAIVER-8 — MILE-113 history rows name paths as they were

## What this covers

`citation.path-exists` reports a path named in MILE-113's revision log that is not in the working tree. Revision-log rows are append-only history, and this record's row names a path as it was when the row was written (source files by crate-relative paths). Rewriting the row to today's path would falsify what happened, and the row cannot be removed. The record's non-history text is already correct. `citation.path-exists` reads the revision log because it sits under the References heading; the waiver expires so it is revisited when the rule can skip the revision log.

## References

- `docs/milestones/MILE-113-an-opt-in-rule-flagging-two-keys-in-one-record-differing-only-by-case-composable-alongside-exact-match-duplicate-detection.md` — the record whose text is covered.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed with the own-repo rollout of `citation.path-exists` (`MILE-143`). | **substantive** |
