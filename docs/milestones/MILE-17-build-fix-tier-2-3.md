---
Status: Planned
Stable-Id: 01M1Y5JN3DVH6232BWZW9DA4AJ
Phase: '2'
Track: embodiment-model
Implements: ADR-15
Blocked-on: —
---
# 17 — Build fix Tier 2/3

## What

The agentic, judgment-requiring repair tiers beyond Tier 1's mechanical recomputation.

## Why

Named in RFC-8/ADR-15 as future scope, not yet built.

## Done means

- Each repair tier beyond Tier 1 is decided in an ADR.
- Each repair tier beyond Tier 1 is built as a declared option of `urzua fix` that never writes without a mechanical verifier accepting the result.
- A planted repair that fails verification is refused and reported.
- The tier is exercised against urzua's own records.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
