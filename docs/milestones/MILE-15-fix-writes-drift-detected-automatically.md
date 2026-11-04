---
Status: Done
Stable-Id: 01M1Y5JM1JV2Z4AAV93DKFYS1P
Phase: '1'
Track: embodiment-model
Implements: ADR-32
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/fix.rs, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs
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
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #151: `urzua fix` detects and `--apply` writes `Drift detected` from the drifted set `check` computes. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
