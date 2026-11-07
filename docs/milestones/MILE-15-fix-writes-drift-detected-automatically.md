---
Status: Done
Stable-Id: 01M1Y5JM1JV2Z4AAV93DKFYS1P
Phase: '1'
Track: embodiment-model
Implements: ADR-32
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/fix.rs, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#detect_reports_drift_detected_as_the_computed_value@sha256:4a897f9195c546a3736948bded974d5dd81c8f2654aec9edc2d2b7014c9184cb, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#apply_writes_drift_detected_and_a_revision_log_row@sha256:8ad2485f3747908a7e98a396ab4fcbb8d1d4899819dc5c4a6059430ff6c08bcc, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#apply_is_idempotent_and_check_agrees_afterwards@sha256:47ef92e836e36f38fe740452447d14795b59d44f8790e3b15535da058ed00ace, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs
---
# 15 — fix writes Drift detected automatically

## What

Wire the drift-detection HashSet already computed for check into fix::detect_repairs/apply_repair so Drift detected becomes a tool-writable value.

## Why

ADR-32 explicitly named this as separate follow-up work, not implied by drift detection landing in check alone.

## Done means

- `urzua fix` detects `Drift detected` from the drifted-record set `check` computes. — evidence: code:rust/crates/urzua-core/src/fix.rs, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#detect_reports_drift_detected_as_the_computed_value@sha256:4a897f9195c546a3736948bded974d5dd81c8f2654aec9edc2d2b7014c9184cb
- `urzua fix --apply` writes `Drift detected` with a revision-log row. — evidence: code:rust/crates/urzua-core/src/fix.rs, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#apply_writes_drift_detected_and_a_revision_log_row@sha256:8ad2485f3747908a7e98a396ab4fcbb8d1d4899819dc5c4a6059430ff6c08bcc
- Applying twice is a no-op and `check` agrees afterwards. — evidence: test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#apply_is_idempotent_and_check_agrees_afterwards@sha256:47ef92e836e36f38fe740452447d14795b59d44f8790e3b15535da058ed00ace
- Tests cover detection, application and idempotence (`fix_drift_integration.rs`). — evidence: test:rust/crates/urzua-cli/tests/fix_drift_integration.rs

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-03 | Built: `fix` detects and `--apply` writes `Drift detected`, using the drifted-record set `check` computes (`SPEC-8` `0.6`). **Why:** the work this milestone planned; `Status` is left for a human to move. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #151: `urzua fix` detects and `--apply` writes `Drift detected` from the drifted set `check` computes. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
