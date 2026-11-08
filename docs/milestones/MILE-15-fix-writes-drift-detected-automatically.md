---
Status: Done
Stable-Id: 01M1Y5JM1JV2Z4AAV93DKFYS1P
Phase: '1'
Track: embodiment-model
Implements: ADR-32
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/fix.rs@sha256:7d8ab5713ec8cf373090381ad76bc27aee6183fb4c6a887a7d655839404bed62, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#detect_reports_drift_detected_as_the_computed_value@sha256:4a897f9195c546a3736948bded974d5dd81c8f2654aec9edc2d2b7014c9184cb, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#apply_writes_drift_detected_and_a_revision_log_row@sha256:8ad2485f3747908a7e98a396ab4fcbb8d1d4899819dc5c4a6059430ff6c08bcc, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#apply_is_idempotent_and_check_agrees_afterwards@sha256:47ef92e836e36f38fe740452447d14795b59d44f8790e3b15535da058ed00ace, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs@sha256:ff69343ca324400bde79c266372f9a213728d4d4ef8202beca6e12e54aa35352
---
# 15 — fix writes Drift detected automatically

## What

Wire the drift-detection HashSet already computed for check into fix::detect_repairs/apply_repair so Drift detected becomes a tool-writable value.

## Why

ADR-32 explicitly named this as separate follow-up work, not implied by drift detection landing in check alone.

## Done means

- `urzua fix` detects `Drift detected` from the drifted-record set `check` computes. — evidence: code:rust/crates/urzua-core/src/fix.rs@sha256:7d8ab5713ec8cf373090381ad76bc27aee6183fb4c6a887a7d655839404bed62, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#detect_reports_drift_detected_as_the_computed_value@sha256:4a897f9195c546a3736948bded974d5dd81c8f2654aec9edc2d2b7014c9184cb
- `urzua fix --apply` writes `Drift detected` with a revision-log row. — evidence: code:rust/crates/urzua-core/src/fix.rs@sha256:7d8ab5713ec8cf373090381ad76bc27aee6183fb4c6a887a7d655839404bed62, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#apply_writes_drift_detected_and_a_revision_log_row@sha256:8ad2485f3747908a7e98a396ab4fcbb8d1d4899819dc5c4a6059430ff6c08bcc
- Applying twice is a no-op and `check` agrees afterwards. — evidence: test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#apply_is_idempotent_and_check_agrees_afterwards@sha256:47ef92e836e36f38fe740452447d14795b59d44f8790e3b15535da058ed00ace
- Tests cover detection, application and idempotence (`fix_drift_integration.rs`). — evidence: test:rust/crates/urzua-cli/tests/fix_drift_integration.rs@sha256:ff69343ca324400bde79c266372f9a213728d4d4ef8202beca6e12e54aa35352

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-03 | Built: `fix` detects and `--apply` writes `Drift detected`, using the drifted-record set `check` computes (`SPEC-8` `0.6`). **Why:** the work this milestone planned; `Status` is left for a human to move. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #151: `urzua fix` detects and `--apply` writes `Drift detected` from the drifted set `check` computes. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/fix.rs@sha256:c9cd87e7b00e, rust/crates/urzua-cli/tests/fix_drift_integration.rs@sha256:ff69343ca324 Reason: Pinned the files as they are now after checking each against the record it evidences: fix.rs and fix_drift_integration.rs still detect, apply and keep idempotent the Drift detected write (MILE-15), discovery.rs still honours exclude_files (MILE-150), config.yaml still declares the identity pattern and field set that urzua new follows, the indexes and index regeneration, the declared placeholder vocabulary, provisional relation claims, per-type relation pairs, external_ids, date_order and the exclude_paths globs (MILE-151, MILE-152, MILE-153, MILE-154, MILE-155, MILE-156, MILE-157, MILE-158), commands/index.rs still regenerates the declared indexes (MILE-152), field_state.rs and status_model.rs still read the declared pending vocabulary (MILE-153), config.rs still reads the relation-pair declaration and the corpus-zero expectation files still exist for the index, status, relation, citation and date rule families (MILE-152 to MILE-157). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` c9cd87e7b00e -> 88c815c0639a. Reason: Read the diff of each pinned file: core fix.rs moved apply_rebaseline's evidence-line lookup and revision-row tail into shared helpers (evidence_edit_lines, append_log_rows, revision_log_refusal) that apply_reverify also calls, with the same rows and error text; cli commands/fix.rs gained validate_reason, back_pointer_evidence, into_failures and identity_notices extracted from fix and reverify with the same behaviour; core reverify.rs calls the helpers and its first-pin matcher no longer splices into a longer anchored locator; cli commands/reverify.rs now refuses a record with no usable revision log in detect, refuses any non-human actor at an error-level period, and honours --ids under --force. Each claim these records make about those files still holds. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/fix.rs` 88c815c0639a -> 7d8ab5713ec8. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: fix.rs: split detect_repairs into the RepairScan methods embodiment and status, and detect_rebaselines into stale_pins and stale_item_lines. Behaviour unchanged, outputs byte-identical (check, check --base, fix --rebaseline, fix and reverify JSON compared before and after). **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
