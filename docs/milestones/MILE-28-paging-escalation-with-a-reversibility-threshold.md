---
Status: Planned
Stable-Id: 01M1Y5JV2770NVZ2FJE3WCX02H
Phase: '3'
Track: escalation
Implements: —
Blocked-on: —
---
# 28 — Paging/escalation with a reversibility threshold

## What

Route a record entering a state (Proposed, Blocked, Drift detected) to its owner, gated by how reversible the underlying decision is.

## Why

Named as the pitch's urgency driver; RFC-1 and RFC-5 both explicitly list the paging mechanism itself as a non-goal.

## Done means

A record entering a declared state (Proposed, Blocked, Drift detected) is routed to its owner by a configured channel, gated by a declared reversibility threshold; a fixture shows a high-reversibility change not paged and a low one paged, and the routing is dry-run safe; the paging mechanism is decided in an ADR first, since `RFC-1` and `RFC-5` list it as a non-goal.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |