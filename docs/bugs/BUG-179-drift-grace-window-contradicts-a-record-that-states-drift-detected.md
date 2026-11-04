---
Stable-Id: 01M6J948WM3NNBZA8WQN63E26M
Status: Open
Found-in: 'running make records after MILE-129 merged with drift_grace_days: 14 (14 warnings, 2026-11-04)'
Regression-test: 'rust/crates/urzua-cli/tests/evidence_pins_integration.rs::a_stated_drift_detected_is_not_contradicted_by_fresh_drift_inside_the_grace_window_observed_failing, rust/crates/urzua-cli/tests/evidence_pins_integration.rs::fix_does_not_flip_a_stated_drift_detected_while_the_drift_is_inside_the_grace_window_observed_failing, rust/crates/urzua-core/src/rules/embodiment.rs::fresh_drift_accepts_either_the_tier_or_drift_detected_observed_failing, rust/crates/urzua-core/src/fix.rs::fresh_drift_leaves_a_stated_drift_detected_alone_observed_failing'
---
# 179 — Drift grace window contradicts a record that states Drift detected

## What was wrong

With `drift_grace_days` set, drift newer than the window was treated as not drifted, so
`embodiment.consistency` computed the tier value (for example `Verified`). A record that honestly
states `Embodiment: Drift detected` was then reported as a mismatch ("stated Drift detected,
computed Verified"), and `urzua fix` would have rewritten it to the tier value. This repository
showed 14 such warnings (ADR-16, 17, 19, 20, 23, 31, 37, 42, 46, 52, 54 and SPEC-2, 3, 5).

The grace window is meant only to stop reporting drift for a fresh change. It must not contradict a
record that already says so. Evidence that drifted inside the window now agrees with either its tier
value or `Drift detected`; evidence that drifted outside the window still computes `Drift detected`,
and `urzua fix` leaves a stated `Drift detected` alone while the drift is only inside the window.

## Why nothing caught it

The `MILE-129` tests covered a record stating its tier (silent inside the window, reported outside
it) and never a record that already stated `Drift detected` when the window began to hold drift back.

## References

Built under `MILE-129`; documented in `SPEC-2` (`embodiment.consistency`).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** `make records` reported 14 warnings contradicting records that state `Drift detected`, once `drift_grace_days: 14` was set. | **substantive** |
