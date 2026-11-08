---
Status: Done
Stable-Id: 01M1YNNBHC3AAT0QEJXEZR3W2F
Phase: '0'
Track: header-format
Implements: ADR-27
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/commands/new.rs@sha256:491185f4e11dd5838880962c7368c1976e28cf424cf5001f23452fe7ce73a855, test:rust/crates/urzua-cli/tests/new_integration.rs#new_emits_yaml_frontmatter_when_configured_even_with_a_blockquote_template_present@sha256:abbf083803b8a4fe62e6e7b933ffdec9e9964180b452f0a00334eaca302cfbd0, test:rust/crates/urzua-cli/tests/new_integration.rs#new_always_writes_a_stable_id_and_warns_when_the_closed_field_set_omits_it_observed_failing@sha256:4654a7fc1fb2b5af69df0dac9b6901444e3e348eaea69860906e0b706fb04b76, code:rust/crates/urzua-core/src/new_record.rs@sha256:21f5012c2b5ff34c2492d7ad9ca7c0a0a35fdc743df9fc76ae124a8210430e67, test:rust/crates/urzua-cli/tests/new_integration.rs#new_fills_differently_named_date_and_author_fields_from_the_declaration@sha256:21637897a097da1db8c5af427984af8a6077f9e5a6a43a28934ff9c8672dea6d, test:rust/crates/urzua-cli/tests/new_integration.rs#new_numbers_from_the_declared_pattern@sha256:3ebc2e4d6db0bd29621bb94d8c9a16470bc08b8c82d17d5938fc44c68e2a1099, test:rust/crates/urzua-cli/tests/new_integration.rs#new_over_a_padded_identity_pattern_passes_check_with_no_edit_observed_failing@sha256:037056b7b5b15eb088207e5d6dad10da14802a378f408054e5a31dbd22c3fac0, spec:docs/specs/SPEC-12-urzua-new.md
---
# 60 — Build urzua new

## What

`urzua new <type> [title]`: fills the type's checked-in template (or synthesizes YAML frontmatter if the type declares that shape with no template) with a fresh stable ID, resolved author, and today's date. Never asks for a number.

## Why

Every other command validates or reports on records that already exist; nothing created one. A tool whose whole point is a well-formed record shouldn't leave record creation to a human copy-pasting the last one.

## Done means

- `urzua new <type> [title]` fills the type's checked-in template with a fresh stable id, a resolved author and today's date. — evidence: code:rust/crates/urzua-cli/src/commands/new.rs@sha256:491185f4e11dd5838880962c7368c1976e28cf424cf5001f23452fe7ce73a855, test:rust/crates/urzua-cli/tests/new_integration.rs#new_emits_yaml_frontmatter_when_configured_even_with_a_blockquote_template_present@sha256:abbf083803b8a4fe62e6e7b933ffdec9e9964180b452f0a00334eaca302cfbd0, test:rust/crates/urzua-cli/tests/new_integration.rs#new_always_writes_a_stable_id_and_warns_when_the_closed_field_set_omits_it_observed_failing@sha256:4654a7fc1fb2b5af69df0dac9b6901444e3e348eaea69860906e0b706fb04b76
- `urzua new <type> [title]` synthesizes YAML frontmatter when the type declares that shape with no template. — evidence: code:rust/crates/urzua-core/src/new_record.rs@sha256:21f5012c2b5ff34c2492d7ad9ca7c0a0a35fdc743df9fc76ae124a8210430e67, test:rust/crates/urzua-cli/tests/new_integration.rs#new_fills_differently_named_date_and_author_fields_from_the_declaration@sha256:21637897a097da1db8c5af427984af8a6077f9e5a6a43a28934ff9c8672dea6d
- `urzua new` never asks for a number. — evidence: test:rust/crates/urzua-cli/tests/new_integration.rs#new_numbers_from_the_declared_pattern@sha256:3ebc2e4d6db0bd29621bb94d8c9a16470bc08b8c82d17d5938fc44c68e2a1099
- `new_integration.rs` covers both paths and the written record passes `check` (`SPEC-12`). — evidence: test:rust/crates/urzua-cli/tests/new_integration.rs#new_over_a_padded_identity_pattern_passes_check_with_no_edit_observed_failing@sha256:037056b7b5b15eb088207e5d6dad10da14802a378f408054e5a31dbd22c3fac0, spec:docs/specs/SPEC-12-urzua-new.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-06 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/commands/new.rs@sha256:5b61c497ff7e, rust/crates/urzua-core/src/new_record.rs@sha256:21f5012c2b5f Reason: Pinned the files as they are now after checking each against the record it evidences: report.rs, config.rs and runner.rs still carry the info level (MILE-46), init.rs still proposes a config in adopt mode (MILE-57), doctor.rs, the Makefile, both git hooks and checks.yml still report and wire the gate (MILE-58), waiver.rs, the waiver template and the WAIVER-0035 record still define and hold a reviewed exception (MILE-59), new.rs and new_record.rs still create the next record (MILE-60), both fix files and fix_drift_integration.rs still detect and apply (MILE-61), pointer.rs, audit.rs and the corpus-b expectation still run the audit that reports one-sided supersession (MILE-62), explain.rs and graph.rs still print the governing records and the edges (MILE-63), the corpus-a expectation still shows a config naming the removed bold-list shape rejected, which is what the milestone's item says (MILE-65), and migrate.rs still reports what a newly required field would break without writing (MILE-68). MILE-46's revision table had a blank line between its header and its rows, which hid every row from the parser, so that line was removed before pinning. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/new.rs` 5b61c497ff7e -> 491185f4e11d. Reason: split new::run into lookup_record_type, scan_pull_requests, number_record, build_filename, render_record and write_new_file (new.rs); behaviour unchanged, outputs byte-identical **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
