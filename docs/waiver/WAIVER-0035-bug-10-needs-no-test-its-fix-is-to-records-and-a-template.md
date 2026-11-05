---
Stable-Id: 01M6NKGNCA5E0S5CBTTN58DHJN
Rule: status.delivery-agreement
Scope: docs/bugs/BUG-10-parent-field-defaults-to-spec-1-across-most-specs-without-a-real-relationship.md
Reason: "The fix set `Parent` to `\u2014` on seven spec records and removed the hardcoded `Parent: SPEC-1 (v0 CLI).` boilerplate from `.urzua/templates/spec.md`. It changed records and a template, no code. `pointer.resolution` only checks that a `Parent` resolves, never that it is the right record (`ADR-40`), so no test can exercise the corrected values; the record notes they were checked by hand against each spec's own lineage."
Category: docs-or-workflow-only
---
# WAIVER-0035 — BUG-10 needs no test: its fix is to records and a template

## What this covers

`status.delivery-agreement` reports `BUG-10`: its `Status` is `Fixed`, but no `Realized-by` evidence computes to a delivered tier. The fix set `Parent` to `—` on seven spec records and removed the hardcoded `Parent: SPEC-1 (v0 CLI).` boilerplate from `.urzua/templates/spec.md`. It changed records and a template, no code. `pointer.resolution` only checks that a `Parent` resolves, never that it is the right record (`ADR-40`), so no test can exercise the corrected values; the record notes they were checked by hand against each spec's own lineage.

The waiver has no `Expires`: the cause is a permanent fact about the record, and the `permanent` declaration in `.urzua/config.yaml` allows it under `docs-or-workflow-only`. It is reported by `waiver.matches-nothing` if the record gains evidence and the finding stops.

## References

- `docs/bugs/BUG-10-parent-field-defaults-to-spec-1-across-most-specs-without-a-real-relationship.md` — the record whose status this waives.
- `ADR-84` — the rule; `ADR-86` — permanent waivers under a declared category; `MILE-193`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed with the `ADR-84` build (`status.delivery-agreement` on bugs). **Why:** the record is `Fixed` with a reason no test can cover, stated in `Reason`. | **substantive** |
