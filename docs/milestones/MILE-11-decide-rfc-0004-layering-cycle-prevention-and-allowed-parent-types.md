---
Status: Planned
Stable-Id: 01M1Y5JHXAF8G3CZTGB6D63WF3
Phase: '1'
Track: governance-process
Kind: decision
Implements: RFC-4
Blocked-on: —
---
# 11 — Decide RFC-4: layering, cycle prevention, and allowed-parent-types

## What

Turn RFC-4's addendum (cycle prevention for Implements/Derives-from, an optional per-profile allowed-parent-types constraint) into a real decision.

## Why

Surfaced by exercising the generic relationship mechanism with a fourth record type (milestone) -- cheap to decide now, expensive to retrofit once real corpora have cross-type links.

## Done means

- An ADR records the decision on cycle prevention for `Implements` and `Derives-from` and on an optional per-profile allowed-parent-types constraint (adopted or rejected, with the reason).
- `RFC-4` points at that ADR.
- If adopted, the rule or option is declared in `.urzua/config.yaml`.
- If adopted, the rule or option is exercised against a planted cycle and a planted disallowed parent in urzua's own corpus tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-07 | `Kind: decision` added. **Why:** maintainer decision of 2026-11-07 (`ADR-96` amendment, milestone kinds): this milestone records a decision, so it is judged against a decided record (an Accepted decision or a Shipped spec) rather than a built tier. Its `Status` is computed from its Done means items by `urzua fix`, not set by hand. | **structural** |
