---
Status: Done
Stable-Id: 01M1Y5JGVDEQ3FGKG31KW7DPX1
Phase: '0'
Track: roadmap-tracking
Implements: ADR-35
Blocked-on: —
Realized-by: spec:docs/specs/SPEC-9-the-bug-record-type.md, code:.urzua/config.yaml
---
# 9 — Add the bug record type

## What

A configured record type requiring a named regression test, distinct from milestone's prospective planning fields.

## Why

A milestone's schema doesn't fit a retrospective defect well -- the one real governance gain (a required, checkable pointer to the proving test) needed its own type.

> **Revision log**
>
> | Date | Change | Class |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> |---|---|---|
| 2026-09-07 | Initial record: already complete at creation time. | structural |
