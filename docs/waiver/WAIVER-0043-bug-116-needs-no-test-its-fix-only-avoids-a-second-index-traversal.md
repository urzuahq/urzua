---
Stable-Id: 01M6NKGXMKHDS56JGFAVX10P2K
Rule: status.delivery-agreement
Scope: docs/bugs/BUG-116-identity-collision-rebuilds-the-shared-record-index-independently.md
Reason: "Fixed by building the record index and its collisions list once in `check.rs` and passing the collisions to `identity_collision`, instead of rebuilding the same index. The two traversals produced the same index, so output is unchanged and a test could not tell them apart without counting traversals, which the engine does not expose."
Category: performance-only
---
# WAIVER-0043 — BUG-116 needs no test: its fix only avoids a second index traversal

## What this covers

`status.delivery-agreement` reports `BUG-116`: its `Status` is `Fixed`, but no `Realized-by` evidence computes to a delivered tier. Fixed by building the record index and its collisions list once in `check.rs` and passing the collisions to `identity_collision`, instead of rebuilding the same index. The two traversals produced the same index, so output is unchanged and a test could not tell them apart without counting traversals, which the engine does not expose.

The waiver has no `Expires`: the cause is a permanent fact about the record, and the `permanent` declaration in `.urzua/config.yaml` allows it under `performance-only`. It is reported by `waiver.matches-nothing` if the record gains evidence and the finding stops.

## References

- `docs/bugs/BUG-116-identity-collision-rebuilds-the-shared-record-index-independently.md` — the record whose status this waives.
- `ADR-84` — the rule; `ADR-86` — permanent waivers under a declared category; `MILE-193`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed with the `ADR-84` build (`status.delivery-agreement` on bugs). **Why:** the record is `Fixed` with a reason no test can cover, stated in `Reason`. | **substantive** |
