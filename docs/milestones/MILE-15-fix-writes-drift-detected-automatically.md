# 15 — fix writes Drift detected automatically

> Status: Planned
> Stable-Id: 01M1Y5JM1JV2Z4AAV93DKFYS1P
> Phase: 1
> Track: embodiment-model
> Implements: ADR-32
> Blocked-on: —

## What

Wire the drift-detection HashSet already computed for check into fix::detect_repairs/apply_repair so Drift detected becomes a tool-writable value.

## Why

ADR-32 explicitly named this as separate follow-up work, not implied by drift detection landing in check alone.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
