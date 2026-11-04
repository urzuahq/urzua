---
Status: Planned
Stable-Id: 01M1Y5JMJMDDAP1WWSK37BTW5T
Phase: '1'
Track: embodiment-model
Implements: RFC-5
Blocked-on: —
---
# 16 — Build RFC-5's full claim graph

## What

AND/OR composite claims, weakest-link confidence, cycle detection, promoted shared nodes -- the fuller model ADR-18's MVP deliberately narrowed.

## Why

Staged intentionally: ships once a real multi-part-AND case is found needing it, not before.

## Done means

The claim graph supports AND and OR composite claims with weakest-link confidence, reports a cycle, and promotes a shared node, each with a planted-violation fixture; it is built only once a real multi-part-AND case in a corpus needs it, and that case is cited here before work starts.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |