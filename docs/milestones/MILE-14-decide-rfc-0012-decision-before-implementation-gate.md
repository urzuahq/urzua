---
Status: Planned
Stable-Id: 01M1Y5JKGMK9VX40WZQFBQ6TTR
Phase: '1'
Track: governance-process
Implements: RFC-12
Blocked-on: —
---
# 14 — Decide RFC-12: decision-before-implementation gate

## What

Refuse the merge, not the edit.

## Why

Still Draft; genuinely undecided.

## Done means

An ADR decides whether merges are refused when a change lands ahead of its decision (the merge, not the edit), names the CI step that enforces it, and `RFC-12` points at it; if adopted, a planted change citing an unaccepted decision fails that step in urzua's own `.github/workflows/` and a decided one passes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |