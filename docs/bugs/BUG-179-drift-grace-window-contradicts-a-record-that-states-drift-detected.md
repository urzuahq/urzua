---
Stable-Id: 01M6J948WM3NNBZA8WQN63E26M
Status: Fixed
Found-in: 'running make records after MILE-129 merged with drift_grace_days: 14 (14 warnings, 2026-11-04)'
Realized-by: test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_stated_drift_detected_is_not_contradicted_by_fresh_drift_inside_the_grace_window_observed_failing@sha256:73f4821eea449db7a098ccfcdbb6041a36c0480512496718c3cc15b0dbb2e604, test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#fix_does_not_flip_a_stated_drift_detected_while_the_drift_is_inside_the_grace_window_observed_failing@sha256:6e1ef9e2802a846e4f9ab0bdd3cd9ef059e764429adb8e80a9454ea9c689481f, test:rust/crates/urzua-core/src/rules/embodiment.rs#fresh_drift_accepts_either_the_tier_or_drift_detected_observed_failing@sha256:4472415693ec2f84355265fd4c4701c235e4c935ab2f04d12b1d89486eae849c, test:rust/crates/urzua-core/src/fix.rs#fresh_drift_leaves_a_stated_drift_detected_alone_observed_failing@sha256:47a1fe1a9e950d5fd8518169b53da51a3d0d5774386dee84ea83429f8c9a2584
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
> | 2026-11-04 | `Status` set to `Fixed`. Fixed in #182: inside `drift_grace_days`, `embodiment.consistency` accepts either the tier value or a stated `Drift detected`, and `urzua fix` leaves a stated `Drift detected` alone; the `Regression-test` tests were added there. **Why:** the fix merged the day this was filed and the record was never closed. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 4 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs#fresh_drift_leaves_a_stated_drift_detected_alone_observed_failing` (lines 1351-1366) 17d645ed962f -> 47a1fe1a9e95. Reason: Read the diff of each pinned span since the old pin and the claim still holds. fix.rs#fresh_drift_leaves_a_stated_drift_detected_alone_observed_failing: the test helper detect replaced the detect_repairs call, assertions unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
