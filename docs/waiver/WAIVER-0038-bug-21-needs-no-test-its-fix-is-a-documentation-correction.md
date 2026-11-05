---
Stable-Id: 01M6NKGRPHZ9DGCHGKRKJ8ZFG8
Rule: status.delivery-agreement
Scope: docs/bugs/BUG-21-spec-2-documents-an-illustrative-shape-that-doesnt-match-what-check-ships.md
Reason: "Fixed by rewriting `SPEC-2`'s Output contract JSON block and prose to the shipped shape. No mechanical check compares a spec's illustrative JSON with the real output, and the record's Fix section explains why none is built, so no test applies to the correction."
Category: docs-or-workflow-only
---
# WAIVER-0038 — BUG-21 needs no test: its fix is a documentation correction

## What this covers

`status.delivery-agreement` reports `BUG-21`: its `Status` is `Fixed`, but no `Realized-by` evidence computes to a delivered tier. Fixed by rewriting `SPEC-2`'s Output contract JSON block and prose to the shipped shape. No mechanical check compares a spec's illustrative JSON with the real output, and the record's Fix section explains why none is built, so no test applies to the correction.

The waiver has no `Expires`: the cause is a permanent fact about the record, and the `permanent` declaration in `.urzua/config.yaml` allows it under `docs-or-workflow-only`. It is reported by `waiver.matches-nothing` if the record gains evidence and the finding stops.

## References

- `docs/bugs/BUG-21-spec-2-documents-an-illustrative-shape-that-doesnt-match-what-check-ships.md` — the record whose status this waives.
- `ADR-84` — the rule; `ADR-86` — permanent waivers under a declared category; `MILE-193`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed with the `ADR-84` build (`status.delivery-agreement` on bugs). **Why:** the record is `Fixed` with a reason no test can cover, stated in `Reason`. | **substantive** |
