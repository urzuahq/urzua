---
Stable-Id: 01M6JW47AQ6W7550ZN46VXHF2B
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: code:.urzua/config.yaml@sha256:151830e5926d1887be39a723d110744ff5e2ae8c408bb114a90ba3e2b8316a2e, test:rust/crates/urzua-core/src/rules/revision_log.rs#a_declared_heading_identity_finds_a_renamed_section@sha256:0acedbf01cc844c8e0dea10a5d6acc4a0f896879873d74f7d102d5e6a19d202c, test:rust/crates/urzua-core/src/rules/revision_log.rs#prose_mentioning_the_marker_does_not_hide_the_real_log_observed_failing@sha256:b8d897d99a6298763fbfc148e0d2244ebdbd2a85e7a4c5ae926173d1b63e20cd, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-revision-log-rules-run.yaml@sha256:0579495840da8c3e54f410498ed47612dcb2bf723356351c801d9e6e60fa2d54
---
# 167 — The revision-log section identity is declared in configuration, not hardcoded

## What

Replace the literal `**Revision log**` in Rust source with a section identity declared in configuration, defaulting to the current literal.

## Why

`ADR-64` makes config-driven the whole thesis, yet the revision-log section is found by one hardcoded string. A record whose prose contains that literal makes the parser read the wrong table, and a repository that names the section differently cannot use the log at all.

## Done means

- The identity is declared per type in `.urzua/config.yaml`. — evidence: code:.urzua/config.yaml@sha256:151830e5926d1887be39a723d110744ff5e2ae8c408bb114a90ba3e2b8316a2e, test:rust/crates/urzua-core/src/rules/revision_log.rs#a_declared_heading_identity_finds_a_renamed_section@sha256:0acedbf01cc844c8e0dea10a5d6acc4a0f896879873d74f7d102d5e6a19d202c
- A record whose prose mentions the literal elsewhere is not misread. — evidence: test:rust/crates/urzua-core/src/rules/revision_log.rs#prose_mentioning_the_marker_does_not_hide_the_real_log_observed_failing@sha256:b8d897d99a6298763fbfc148e0d2244ebdbd2a85e7a4c5ae926173d1b63e20cd
- The default keeps existing repositories unchanged. — evidence: test:rust/crates/urzua-core/src/rules/revision_log.rs#prose_mentioning_the_marker_does_not_hide_the_real_log_observed_failing@sha256:b8d897d99a6298763fbfc148e0d2244ebdbd2a85e7a4c5ae926173d1b63e20cd
- A fixture covers a renamed section and a prose mention. — evidence: test:rust/crates/urzua-core/src/rules/revision_log.rs#a_declared_heading_identity_finds_a_renamed_section@sha256:0acedbf01cc844c8e0dea10a5d6acc4a0f896879873d74f7d102d5e6a19d202c, test:rust/crates/urzua-core/src/rules/revision_log.rs#prose_mentioning_the_marker_does_not_hide_the_real_log_observed_failing@sha256:b8d897d99a6298763fbfc148e0d2244ebdbd2a85e7a4c5ae926173d1b63e20cd
- The capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests. — evidence: code:.urzua/config.yaml@sha256:151830e5926d1887be39a723d110744ff5e2ae8c408bb114a90ba3e2b8316a2e, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-revision-log-rules-run.yaml@sha256:0579495840da8c3e54f410498ed47612dcb2bf723356351c801d9e6e60fa2d54

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Built: per-type `revision_log` (`marker` or `heading`, default `**Revision log**`) matched as the opening line, outside fences; read by `revision-log.change-class-required`, `urzua fix` and `citation.path-exists`. `skip_tables` (unreleased) became the boolean `skip_revision_log`. Declared on `adr` in this repository's config. Documented in `SPEC-2` and `SPEC-3`. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #213: the revision-log section identity is a per-type declaration. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/revision_log.rs#a_declared_heading_identity_finds_a_renamed_section` (lines 636-660) 4a55788dd6c5 -> 0acedbf01cc8. Reason: Read the diff of each pinned span since the old pin and the claim still holds. revision_log.rs#a_declared_heading_identity_finds_a_renamed_section: the test's config literal gained ..Default::default() for newly added fields, assertions unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: .urzua/config.yaml@sha256:151830e5926d, rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-revision-log-rules-run.yaml@sha256:0579495840da Reason: Pinned the files as they are now after checking each against the record it evidences: config.yaml still declares test_paths, the statuses.pending pattern entries, the pending acknowledgement field behind provisional claims, the index format placeholders, drift_audit_field, the revision-log marker, waiver.matches-nothing and pending_until_required that these milestones dogfood (MILE-159 to MILE-169), commands/fix.rs still takes paths, proposed content and apply without a log (MILE-159), back_pointer.rs still matches test_paths as globs (MILE-160), config.rs still reads the declared pending-acknowledgement field (MILE-163), and the corpus-zero embodiment, relation, index, revision-log, waiver and field expectation files still exist and run this repository's own config. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
