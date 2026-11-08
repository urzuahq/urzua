---
Status: Done
Stable-Id: 01M1YNNC28J6KNP3D6PC6H7SCY
Phase: '0'
Track: embodiment-model
Implements: ADR-15, ADR-19, ADR-20
Blocked-on: —
Realized-by: test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#detect_reports_drift_detected_as_the_computed_value@sha256:4a897f9195c546a3736948bded974d5dd81c8f2654aec9edc2d2b7014c9184cb, code:rust/crates/urzua-cli/src/commands/fix.rs, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#apply_writes_drift_detected_and_a_revision_log_row@sha256:8ad2485f3747908a7e98a396ab4fcbb8d1d4899819dc5c4a6059430ff6c08bcc, code:rust/crates/urzua-core/src/fix.rs, test:rust/crates/urzua-core/src/fix.rs#apply_repair_refuses_without_a_revision_log_section@sha256:2a5602cec009ce585797f165219b8b67e4c20f82e20e3aee555b3dd2f12d386c, test:rust/crates/urzua-cli/tests/fix_revision_log_required_integration.rs#required_by_default_a_missing_section_fails_naming_the_record@sha256:7419d6f0c70487731d386192b0b473d0f2b261857fc9dcb36ae373bbb2d87b35, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs, spec:docs/specs/SPEC-8-urzua-fix.md
---
# 61 — Build urzua fix detect and apply modes

## What

`urzua fix`: detect mode reports fields whose stated value disagrees with what's computed from `Realized-by` evidence; `fix --apply --ids <records> --by <you>` writes the computed value back, one field's line only, with a required identity and an appended revision-log entry. Refuses outright on a record with no revision log.

## Why

`check` can tell you a field is wrong; nothing closed the loop by writing the mechanically-correct value back. Apply mode is gated hard (explicit ids or --force, a resolved identity, an auditable revision-log entry) because a bulk unreviewed write is exactly the failure mode a governance tool can't afford.

## Done means

- `urzua fix` in detect mode lists fields whose stated value disagrees with the value computed from `Realized-by` evidence. — evidence: test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#detect_reports_drift_detected_as_the_computed_value@sha256:4a897f9195c546a3736948bded974d5dd81c8f2654aec9edc2d2b7014c9184cb, code:rust/crates/urzua-cli/src/commands/fix.rs
- `fix --apply --ids <records> --by <you>` writes the computed value to that one field's line only, with a resolved identity and an appended revision-log row. — evidence: test:rust/crates/urzua-cli/tests/fix_drift_integration.rs#apply_writes_drift_detected_and_a_revision_log_row@sha256:8ad2485f3747908a7e98a396ab4fcbb8d1d4899819dc5c4a6059430ff6c08bcc, code:rust/crates/urzua-core/src/fix.rs
- `fix --apply` refuses outright on a record with no revision log. — evidence: test:rust/crates/urzua-core/src/fix.rs#apply_repair_refuses_without_a_revision_log_section@sha256:2a5602cec009ce585797f165219b8b67e4c20f82e20e3aee555b3dd2f12d386c, test:rust/crates/urzua-cli/tests/fix_revision_log_required_integration.rs#required_by_default_a_missing_section_fails_naming_the_record@sha256:7419d6f0c70487731d386192b0b473d0f2b261857fc9dcb36ae373bbb2d87b35
- Without explicit ids `fix --apply` refuses unless forced. — evidence: code:rust/crates/urzua-cli/src/commands/fix.rs
- The behaviour is covered in `fix_drift_integration.rs` and specified in `SPEC-8`. — evidence: test:rust/crates/urzua-cli/tests/fix_drift_integration.rs, spec:docs/specs/SPEC-8-urzua-fix.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-05 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs#apply_repair_refuses_without_a_revision_log_section` (lines 1260-1274) 10de1fe7ebb2 -> 2a5602cec009. Reason: Read the diff of each pinned span since the old pin and the claim still holds. fix.rs#apply_repair_refuses_without_a_revision_log_section: the call gained one argument (the revision-log shape, &Default::default()), and it still asserts apply refuses a record with no revision log. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
