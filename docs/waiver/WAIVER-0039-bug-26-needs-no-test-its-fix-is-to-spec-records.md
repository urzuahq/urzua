---
Stable-Id: 01M6NKGSPXJYD9QDS9V35YFH6K
Rule: status.delivery-agreement
Scope: docs/bugs/BUG-26-five-specs-are-marked-draft-while-their-subject-is-built-and-shipped.md
Reason: "Fixed by moving four specs to `Accepted`, relocating `SPEC-5`'s unbuilt sections to `MILE-114` and correcting its Layout section. Every change is to a spec record. The mechanical detection of a stale spec status is `RFC-28`'s separate, undecided proposal, so no test applies to this fix."
Category: docs-or-workflow-only
---
# WAIVER-0039 — BUG-26 needs no test: its fix is to spec records

## What this covers

`status.delivery-agreement` reports `BUG-26`: its `Status` is `Fixed`, but no `Realized-by` evidence computes to a delivered tier. Fixed by moving four specs to `Accepted`, relocating `SPEC-5`'s unbuilt sections to `MILE-114` and correcting its Layout section. Every change is to a spec record. The mechanical detection of a stale spec status is `RFC-28`'s separate, undecided proposal, so no test applies to this fix.

The waiver has no `Expires`: the cause is a permanent fact about the record, and the `permanent` declaration in `.urzua/config.yaml` allows it under `docs-or-workflow-only`. It is reported by `waiver.matches-nothing` if the record gains evidence and the finding stops.

## References

- `docs/bugs/BUG-26-five-specs-are-marked-draft-while-their-subject-is-built-and-shipped.md` — the record whose status this waives.
- `ADR-84` — the rule; `ADR-86` — permanent waivers under a declared category; `MILE-193`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed with the `ADR-84` build (`status.delivery-agreement` on bugs). **Why:** the record is `Fixed` with a reason no test can cover, stated in `Reason`. | **substantive** |
