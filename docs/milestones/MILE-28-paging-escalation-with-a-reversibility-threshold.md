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

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
