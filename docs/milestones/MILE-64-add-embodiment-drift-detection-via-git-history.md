---
Status: Done
Stable-Id: 01M1YNNDMH0SEJNPEV3Y9ZP1QK
Phase: '0'
Track: embodiment-model
Implements: ADR-32
Blocked-on: —
Realized-by: test:rust/crates/urzua-core/src/rules/embodiment.rs#a_drifted_record_computes_to_drift_detected_regardless_of_tier@sha256:382ca34e713c29eba50cacb1ed4a29cbdecff3658b5dfc542fdc325f5b42f515, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#detect_reports_drift_detected_as_the_computed_value@sha256:4a897f9195c546a3736948bded974d5dd81c8f2654aec9edc2d2b7014c9184cb, code:rust/crates/urzua-cli/src/discovery.rs#compute_drifted_records_at@sha256:10f3bac07ae84e0db72b79ecde8b0e6139a460146dd577495220b81628fe399e, test:rust/crates/urzua-core/src/rules/embodiment.rs#a_record_marked_drifted_produces_a_finding_even_when_verified_matches_its_tier@sha256:36877b16568c10c25de031b1a5a88983eed456e7f9bcc10f79a50eed2cc3e498, test:rust/crates/urzua-cli/tests/fix_drift_git_integration.rs#a_path_scoped_check_blames_only_the_records_in_scope@sha256:fd58d2bed147fea409fe3b7fcafc0469bcef0cbe3f9c3ab8d71bc3c50c87b668, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#a_locator_unchanged_since_the_realized_by_line_is_not_drift@sha256:4facdb8c34b85d39e1dcd0185b262df69cc663975aaf8fef14e1d5f1ebce6295, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#apply_writes_drift_detected_and_a_revision_log_row@sha256:8ad2485f3747908a7e98a396ab4fcbb8d1d4899819dc5c4a6059430ff6c08bcc
---
# 64 — Add Embodiment drift detection via git history

## What

`embodiment.consistency` now detects drift: if a `Realized-by` locator changed, per git history, since the `Realized-by` line was last touched, the expected `Embodiment` is `Drift detected` regardless of tier, and a stated value that disagrees is a finding. No stored hash -- the comparison reads git blame/log directly.

## Why

A back-pointer that's merely present proves someone typed a string once, not that the code still matches. Drift detection is what makes `Realized-by` a claim that can go stale and get caught, not a write-once assertion nobody re-checks.

## Done means

- `embodiment.consistency` computes `Drift detected` as the expected `Embodiment` when a `Realized-by` locator changed in git history since the `Realized-by` line was last touched, whatever the tier. — evidence: test:rust/crates/urzua-core/src/rules/embodiment.rs#a_drifted_record_computes_to_drift_detected_regardless_of_tier@sha256:382ca34e713c29eba50cacb1ed4a29cbdecff3658b5dfc542fdc325f5b42f515, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#detect_reports_drift_detected_as_the_computed_value@sha256:4a897f9195c546a3736948bded974d5dd81c8f2654aec9edc2d2b7014c9184cb, code:rust/crates/urzua-cli/src/discovery.rs#compute_drifted_records_at@sha256:10f3bac07ae84e0db72b79ecde8b0e6139a460146dd577495220b81628fe399e
- `embodiment.consistency` reports a stated value that disagrees with the computed one. — evidence: test:rust/crates/urzua-core/src/rules/embodiment.rs#a_record_marked_drifted_produces_a_finding_even_when_verified_matches_its_tier@sha256:36877b16568c10c25de031b1a5a88983eed456e7f9bcc10f79a50eed2cc3e498
- No hash is stored: the comparison reads git blame and log. — evidence: code:rust/crates/urzua-cli/src/discovery.rs#compute_drifted_records_at@sha256:10f3bac07ae84e0db72b79ecde8b0e6139a460146dd577495220b81628fe399e, test:rust/crates/urzua-cli/tests/fix_drift_git_integration.rs#a_path_scoped_check_blames_only_the_records_in_scope@sha256:fd58d2bed147fea409fe3b7fcafc0469bcef0cbe3f9c3ab8d71bc3c50c87b668
- The integration suite plants both a drifted record and an undrifted one. — evidence: test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#a_locator_unchanged_since_the_realized_by_line_is_not_drift@sha256:4facdb8c34b85d39e1dcd0185b262df69cc663975aaf8fef14e1d5f1ebce6295, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#apply_writes_drift_detected_and_a_revision_log_row@sha256:8ad2485f3747908a7e98a396ab4fcbb8d1d4899819dc5c4a6059430ff6c08bcc

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/fix_drift_git_integration.rs#a_path_scoped_check_blames_only_the_records_in_scope` (lines 177-202) a37b892c69a5 -> fd58d2bed147. Reason: Read the diff of each pinned span since the old pin and the claim still holds. fix_drift_git_integration.rs#a_path_scoped_check_blames_only_the_records_in_scope: the inline git-blame shim was moved into a shared blame_runner helper called with an extra None argument, and the assertions that a scoped check blames only the named record and a whole check blames every record are unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
