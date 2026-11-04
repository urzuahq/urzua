---
Stable-Id: 01M6JRM1SDM9N5R93HKEV09NWC
Rule: citation.resolution
Scope: docs/specs/SPEC-2-urzua-check.md
Reason: "The rule table row for citation.resolution is generated from the rule catalog in the engine source, and its example of a numeric range names a lower bound that is not a record. Rewording it means changing the catalog description in the Rust source and regenerating the table, which is an engine-side change outside this rollout. Reviewed finding by finding; expires so the exception is revisited."
Expires: 2027-11-04
---
# WAIVER-33 — SPEC-2 cites ids that are history or illustration

## What this covers

`citation.resolution` reports ids cited in SPEC-2's prose that resolve to no record. The rule table row for citation.resolution is generated from the rule catalog in the engine source, and its example of a numeric range names a lower bound that is not a record. Rewording it means changing the catalog description in the Rust source and regenerating the table, which is an engine-side change outside this rollout.

## References

- `docs/specs/SPEC-2-urzua-check.md` — the record whose text is covered.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed with the own-repo rollout of `citation.resolution` (`MILE-156`). | **substantive** |
