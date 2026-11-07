---
Stable-Id: 01M6SVNJNSFWAP5N4XVHRW5A5S
Rule: status.delivery-agreement
Scope: docs/milestones/MILE-165-section-heading-aliases-and-reading-a-y-statement-outside-any-section-heading.md
Reason: "MILE-165 is set back to InProgress because its Done means asks for the preamble reading to be exercised by this repository's own records, and no record here carries a Y-statement outside a section. Its Realized-by evidence is real (section.rs and its integration test), so the status rule computes Done; this waiver holds the honest status until a follow-up exercises the reading or the maintainer amends the Done means."
Expires: 2026-12-07
---
# WAIVER-0051 — MILE-165 stays InProgress until the preamble reading is exercised on this repository's records

## What this covers

`status.delivery-agreement` reports `MILE-165`: its `Status` is `InProgress`, but its `Realized-by` evidence computes to a delivered tier. Both are true. The code and its fixtures are built and merged (#222), but the record's Done means also requires the capability to be exercised by this repository's own `.urzua/config.yaml` and records, and its own 2026-11-04 row says the `preamble: true` reading has fixture coverage only. `aliases` is exercised here (the `bug` type); the preamble reading is not, because no record of this repository carries a Y-statement outside a section.

The waiver expires on 2026-12-07 so the question comes back: by then either a record here exercises the reading and `MILE-165` is marked Done by `urzua fix --apply`, or the maintainer amends the Done means.

## References

- `docs/milestones/MILE-165-section-heading-aliases-and-reading-a-y-statement-outside-any-section-heading.md` — the milestone whose status this holds.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Filed. **Why:** `MILE-165` was Done while its own row admitted the preamble reading had fixture coverage only; status follows evidence, and the evidence for that half of the Done means is missing. | **substantive** |
