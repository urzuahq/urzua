---
Stable-Id: 01M6NKH0NMMJ7Q6EW2B2DXM6NF
Rule: status.delivery-agreement
Scope: docs/bugs/BUG-143-relation-target-status-undeclared-rebuilds-declared-fields-per-reference-instead-of-reusing-the-cache.md
Reason: "Fixed by precomputing `declared_fields_by_type(config)` once in `relation_target_status_undeclared` instead of rebuilding the set per reference, the same change as `BUG-134`. Findings are unchanged; only CPU time differs, and no test in the suite measures it."
Category: performance-only
---
# WAIVER-0046 — BUG-143 needs no test: its fix only reuses a cached field set

## What this covers

`status.delivery-agreement` reports `BUG-143`: its `Status` is `Fixed`, but no `Realized-by` evidence computes to a delivered tier. Fixed by precomputing `declared_fields_by_type(config)` once in `relation_target_status_undeclared` instead of rebuilding the set per reference, the same change as `BUG-134`. Findings are unchanged; only CPU time differs, and no test in the suite measures it.

The waiver has no `Expires`: the cause is a permanent fact about the record, and the `permanent` declaration in `.urzua/config.yaml` allows it under `performance-only`. It is reported by `waiver.matches-nothing` if the record gains evidence and the finding stops.

## References

- `docs/bugs/BUG-143-relation-target-status-undeclared-rebuilds-declared-fields-per-reference-instead-of-reusing-the-cache.md` — the record whose status this waives.
- `ADR-84` — the rule; `ADR-86` — permanent waivers under a declared category; `MILE-193`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed with the `ADR-84` build (`status.delivery-agreement` on bugs). **Why:** the record is `Fixed` with a reason no test can cover, stated in `Reason`. | **substantive** |
