---
Stable-Id: 01M6NKH3MGE74RF3CPBHD6SS7S
Rule: status.delivery-agreement
Scope: docs/bugs/BUG-105-the-declared-field-slot-list-is-rebuilt-independently-by-four-or-more-rules.md
Reason: "Fixed by extracting `field_slots` and `pointer_and_narrative_slots` in place of six inline slot constructions. No rule's behaviour changed. The rules' existing suites run the helpers through every caller, but no single test is the evidence for the extraction, so none can honestly be cited."
Category: behaviour-preserving-refactor
---
# WAIVER-0049 — BUG-105 needs no test: its fix is a refactor with no single covering test

## What this covers

`status.delivery-agreement` reports `BUG-105`: its `Status` is `Fixed`, but no `Realized-by` evidence computes to a delivered tier. Fixed by extracting `field_slots` and `pointer_and_narrative_slots` in place of six inline slot constructions. No rule's behaviour changed. The rules' existing suites run the helpers through every caller, but no single test is the evidence for the extraction, so none can honestly be cited.

The waiver has no `Expires`: the cause is a permanent fact about the record, and the `permanent` declaration in `.urzua/config.yaml` allows it under `behaviour-preserving-refactor`. It is reported by `waiver.matches-nothing` if the record gains evidence and the finding stops.

## References

- `docs/bugs/BUG-105-the-declared-field-slot-list-is-rebuilt-independently-by-four-or-more-rules.md` — the record whose status this waives.
- `ADR-84` — the rule; `ADR-86` — permanent waivers under a declared category; `MILE-193`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed with the `ADR-84` build (`status.delivery-agreement` on bugs). **Why:** the record is `Fixed` with a reason no test can cover, stated in `Reason`. | **substantive** |
