---
Stable-Id: 01M6NKGZNZGCEVS83Q4NWG75B9
Rule: status.delivery-agreement
Scope: docs/bugs/BUG-134-declared-slots-for-roles-and-declared-cross-record-value-rebuild-a-type-s-declared-field-set-per-record-instead-of-using-the-cached-by-type-map.md
Reason: "Fixed by having `declared_slots_for_roles` and `declared_cross_record_value` use the per-type declared-field cache instead of rebuilding it per record or per reference. Findings are unchanged; the only difference is CPU time, which no test in the suite measures."
Category: performance-only
---
# WAIVER-0045 — BUG-134 needs no test: its fix only reuses a cached field set

## What this covers

`status.delivery-agreement` reports `BUG-134`: its `Status` is `Fixed`, but no `Realized-by` evidence computes to a delivered tier. Fixed by having `declared_slots_for_roles` and `declared_cross_record_value` use the per-type declared-field cache instead of rebuilding it per record or per reference. Findings are unchanged; the only difference is CPU time, which no test in the suite measures.

The waiver has no `Expires`: the cause is a permanent fact about the record, and the `permanent` declaration in `.urzua/config.yaml` allows it under `performance-only`. It is reported by `waiver.matches-nothing` if the record gains evidence and the finding stops.

## References

- `docs/bugs/BUG-134-declared-slots-for-roles-and-declared-cross-record-value-rebuild-a-type-s-declared-field-set-per-record-instead-of-using-the-cached-by-type-map.md` — the record whose status this waives.
- `ADR-84` — the rule; `ADR-86` — permanent waivers under a declared category; `MILE-193`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed with the `ADR-84` build (`status.delivery-agreement` on bugs). **Why:** the record is `Fixed` with a reason no test can cover, stated in `Reason`. | **substantive** |
