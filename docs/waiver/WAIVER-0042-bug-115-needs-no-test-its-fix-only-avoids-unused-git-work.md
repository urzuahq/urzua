---
Stable-Id: 01M6NKGWN93NYMKX616WH22C09
Rule: status.delivery-agreement
Scope: docs/bugs/BUG-115-compute-drifted-records-runs-unconditionally-even-when-embodiment-consistency-is-off.md
Reason: "Fixed by calling `compute_drifted_records` only when `embodiment.consistency` is not `Off`. When the rule is off nothing reads the result, so findings are identical either way; the only difference is skipped `git` subprocesses, which the suite does not count. The record states a real-corpus run reported identical findings."
Category: performance-only
---
# WAIVER-0042 — BUG-115 needs no test: its fix only avoids unused git work

## What this covers

`status.delivery-agreement` reports `BUG-115`: its `Status` is `Fixed`, but no `Realized-by` evidence computes to a delivered tier. Fixed by calling `compute_drifted_records` only when `embodiment.consistency` is not `Off`. When the rule is off nothing reads the result, so findings are identical either way; the only difference is skipped `git` subprocesses, which the suite does not count. The record states a real-corpus run reported identical findings.

The waiver has no `Expires`: the cause is a permanent fact about the record, and the `permanent` declaration in `.urzua/config.yaml` allows it under `performance-only`. It is reported by `waiver.matches-nothing` if the record gains evidence and the finding stops.

## References

- `docs/bugs/BUG-115-compute-drifted-records-runs-unconditionally-even-when-embodiment-consistency-is-off.md` — the record whose status this waives.
- `ADR-84` — the rule; `ADR-86` — permanent waivers under a declared category; `MILE-193`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed with the `ADR-84` build (`status.delivery-agreement` on bugs). **Why:** the record is `Fixed` with a reason no test can cover, stated in `Reason`. | **substantive** |
