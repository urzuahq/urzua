---
Status: Done
Stable-Id: 01M1YNNBHC3AAT0QEJXEZR3W2F
Phase: '0'
Track: header-format
Implements: ADR-27
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/commands/new.rs, test:rust/crates/urzua-cli/tests/new_integration.rs#new_emits_yaml_frontmatter_when_configured_even_with_a_blockquote_template_present@sha256:abbf083803b8a4fe62e6e7b933ffdec9e9964180b452f0a00334eaca302cfbd0, test:rust/crates/urzua-cli/tests/new_integration.rs#new_always_writes_a_stable_id_and_warns_when_the_closed_field_set_omits_it_observed_failing@sha256:4654a7fc1fb2b5af69df0dac9b6901444e3e348eaea69860906e0b706fb04b76, code:rust/crates/urzua-core/src/new_record.rs, test:rust/crates/urzua-cli/tests/new_integration.rs#new_fills_differently_named_date_and_author_fields_from_the_declaration@sha256:21637897a097da1db8c5af427984af8a6077f9e5a6a43a28934ff9c8672dea6d, test:rust/crates/urzua-cli/tests/new_integration.rs#new_numbers_from_the_declared_pattern@sha256:3ebc2e4d6db0bd29621bb94d8c9a16470bc08b8c82d17d5938fc44c68e2a1099, test:rust/crates/urzua-cli/tests/new_integration.rs#new_over_a_padded_identity_pattern_passes_check_with_no_edit_observed_failing@sha256:037056b7b5b15eb088207e5d6dad10da14802a378f408054e5a31dbd22c3fac0, spec:docs/specs/SPEC-12-urzua-new.md
---
# 60 — Build urzua new

## What

`urzua new <type> [title]`: fills the type's checked-in template (or synthesizes YAML frontmatter if the type declares that shape with no template) with a fresh stable ID, resolved author, and today's date. Never asks for a number.

## Why

Every other command validates or reports on records that already exist; nothing created one. A tool whose whole point is a well-formed record shouldn't leave record creation to a human copy-pasting the last one.

## Done means

- `urzua new <type> [title]` fills the type's checked-in template with a fresh stable id, a resolved author and today's date. — evidence: code:rust/crates/urzua-cli/src/commands/new.rs, test:rust/crates/urzua-cli/tests/new_integration.rs#new_emits_yaml_frontmatter_when_configured_even_with_a_blockquote_template_present@sha256:abbf083803b8a4fe62e6e7b933ffdec9e9964180b452f0a00334eaca302cfbd0, test:rust/crates/urzua-cli/tests/new_integration.rs#new_always_writes_a_stable_id_and_warns_when_the_closed_field_set_omits_it_observed_failing@sha256:4654a7fc1fb2b5af69df0dac9b6901444e3e348eaea69860906e0b706fb04b76
- `urzua new <type> [title]` synthesizes YAML frontmatter when the type declares that shape with no template. — evidence: code:rust/crates/urzua-core/src/new_record.rs, test:rust/crates/urzua-cli/tests/new_integration.rs#new_fills_differently_named_date_and_author_fields_from_the_declaration@sha256:21637897a097da1db8c5af427984af8a6077f9e5a6a43a28934ff9c8672dea6d
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
