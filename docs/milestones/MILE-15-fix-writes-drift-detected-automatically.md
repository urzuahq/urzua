---
Status: Planned
Stable-Id: 01M1Y5JM1JV2Z4AAV93DKFYS1P
Phase: '1'
Track: embodiment-model
Implements: ADR-32
Blocked-on: —
---
# 15 — fix writes Drift detected automatically

## What

Wire the drift-detection HashSet already computed for check into fix::detect_repairs/apply_repair so Drift detected becomes a tool-writable value.

## Why

ADR-32 explicitly named this as separate follow-up work, not implied by drift detection landing in check alone.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-03 | Built: `fix` detects and `--apply` writes `Drift detected`, using the drifted-record set `check` computes (`SPEC-8` `0.6`). **Why:** the work this milestone planned; `Status` is left for a human to move. | **substantive** |
