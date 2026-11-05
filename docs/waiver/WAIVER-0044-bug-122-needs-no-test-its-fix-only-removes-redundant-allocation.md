---
Stable-Id: 01M6NKGYKN0EA6RGZ2H5FF4RCC
Rule: status.delivery-agreement
Scope: docs/bugs/BUG-122-census-records-clones-and-sorts-a-pathbuf-per-examined-slot-instead-of-per-record.md
Reason: "Fixed by collecting examined record paths incrementally in a `BTreeSet` in `census_records` instead of cloning one `PathBuf` per slot and sorting at the end. The examined set is the same; the saving is allocation and sort time that this repository's corpus is too small to measure, and the record states no benchmark is added."
Category: performance-only
---
# WAIVER-0044 — BUG-122 needs no test: its fix only removes redundant allocation

## What this covers

`status.delivery-agreement` reports `BUG-122`: its `Status` is `Fixed`, but no `Realized-by` evidence computes to a delivered tier. Fixed by collecting examined record paths incrementally in a `BTreeSet` in `census_records` instead of cloning one `PathBuf` per slot and sorting at the end. The examined set is the same; the saving is allocation and sort time that this repository's corpus is too small to measure, and the record states no benchmark is added.

The waiver has no `Expires`: the cause is a permanent fact about the record, and the `permanent` declaration in `.urzua/config.yaml` allows it under `performance-only`. It is reported by `waiver.matches-nothing` if the record gains evidence and the finding stops.

## References

- `docs/bugs/BUG-122-census-records-clones-and-sorts-a-pathbuf-per-examined-slot-instead-of-per-record.md` — the record whose status this waives.
- `ADR-84` — the rule; `ADR-86` — permanent waivers under a declared category; `MILE-193`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed with the `ADR-84` build (`status.delivery-agreement` on bugs). **Why:** the record is `Fixed` with a reason no test can cover, stated in `Reason`. | **substantive** |
