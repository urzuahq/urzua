# 0028 — Paging/escalation with a reversibility threshold

> Status: Planned
> Stable-Id: 01M1Y5JV2770NVZ2FJE3WCX02H
> Phase: 1
> Track: escalation
> Implements: —

## What

Route a record entering a state (Proposed, Blocked, Drift detected) to its owner, gated by how reversible the underlying decision is.

## Why

Named as the pitch's urgency driver; RFC-0001 and RFC-0005 both explicitly list the paging mechanism itself as a non-goal.

## Blocked on

—

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
