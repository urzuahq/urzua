---
Status: Planned
Stable-Id: 01M1Y5JSEZP2V2MSTE3AX8762W
Phase: '1'
Track: review-workflow
Implements: —
Blocked-on: —
---
# 25 — ARB-style review queue with dissent preserved as first-class

## What

A tiered/SLA'd review queue projection over records, with objections preserved rather than deleted when overruled.

## Why

Flagged as a strong steal-candidate pattern; no RFC covers a review-queue projection or an objections model today.

## Done means

- A review queue projection lists records awaiting review ordered by tier and SLA.
- An objection to a decision is kept as a record with its author and date when overruled, and never deleted.
- `check` reports an overdue item.
- A fixture covers an overruled objection surviving.
- The queue is exercised against urzua's own records.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
