---
Status: Done
Stable-Id: 01M1Y5JEPZRBT40QA2DEXT2CM2
Phase: '0'
Track: section-checks
Implements: RFC-17
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/section.rs, test:rust/crates/urzua-core/src/rules/section.rs#a_missing_phrase_is_named@sha256:d42ad3eea79762fa55ca9c31fed8aadb3c4e49b0a5f3e13770711c4b3fd10907, test:rust/crates/urzua-core/src/rules/section.rs#matching_is_on_word_boundaries@sha256:7b1bbfa391727007c7c27f9b5e0f355367a541615d240a5d49f052be1d36d3c5, test:rust/crates/urzua-core/src/rules/section.rs#a_marker_with_no_block_is_its_own_error@sha256:69bac708240d63172da05f254707a1659f73a47d058f560527779087e9e614a8, test:rust/crates/urzua-core/src/rules/section.rs#a_complete_y_statement_after_the_marker_passes@sha256:8fe3dc6a901a930ef436649c6bf22644e3f16b4992b37fc60aa7889983468391, test:rust/crates/urzua-cli/tests/section_rules_integration.rs#declared_rules_report_each_failure_and_count_slots@sha256:fd72db5c46d6b283f04a776dfb01e5fec3d7b8e2e28a7a6dcf2fac68e22c5376
---
# 5 — Build the y-statement content shape

## What

Extract a Decision section's TL;DR block and check it contains the five Y-statement structural phrases, word-boundary matched, with a distinct error when the marker exists but no extractable block follows.

## Why

The first real content shape, proving the section-parser design; validated against two independent real implementations that converged on the same extraction/matching approach.

## Done means

- A required section can declare `shape: y-statement` with a marker, and the opt-in rule `section.shape` reports a section whose extracted block lacks any of the five phrases (word-boundary matched). — evidence: code:rust/crates/urzua-core/src/rules/section.rs, test:rust/crates/urzua-core/src/rules/section.rs#a_missing_phrase_is_named@sha256:d42ad3eea79762fa55ca9c31fed8aadb3c4e49b0a5f3e13770711c4b3fd10907, test:rust/crates/urzua-core/src/rules/section.rs#matching_is_on_word_boundaries@sha256:7b1bbfa391727007c7c27f9b5e0f355367a541615d240a5d49f052be1d36d3c5
- `section.shape` reports a marker with no block following it as a distinct finding. — evidence: test:rust/crates/urzua-core/src/rules/section.rs#a_marker_with_no_block_is_its_own_error@sha256:69bac708240d63172da05f254707a1659f73a47d058f560527779087e9e614a8
- A fixture covers both failures and a passing block. — evidence: test:rust/crates/urzua-core/src/rules/section.rs#a_complete_y_statement_after_the_marker_passes@sha256:8fe3dc6a901a930ef436649c6bf22644e3f16b4992b37fc60aa7889983468391, test:rust/crates/urzua-cli/tests/section_rules_integration.rs#declared_rules_report_each_failure_and_count_slots@sha256:fd72db5c46d6b283f04a776dfb01e5fec3d7b8e2e28a7a6dcf2fac68e22c5376

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Reshaped by `ADR-53`. **Why:** filed as "build the y-statement content shape" when a shape meant a Rust function. `RFC-33` makes a shape a parameter to `pattern`, so the work is a declared shape plus whatever `pattern` needs to support it -- not a rule. Blocked on the document model, which is what locates a Decision section's TL;DR block in the first place. | **substantive** |
> | 2026-09-17 | `Blocked-on` now names the blocking record rather than describing it. **Why:** it held a sentence (*'Milestone: Build the closed section-parser...'*) which is legal -- `Blocked-on` is a declared `narrative_field`, and prose is what those are for, since "blocked on a decision about X" is a real value no pointer expresses. Changed because in *these three* cases the prose described exactly one existing record, so naming it lets `narrative-field.stale` see the dependency and report when the target goes terminal. Not a defect being fixed; a narrative value replaced by a more useful one. | **structural** |
> | 2026-09-18 | `Blocked-on` now names `MILE-98`. **Why:** the declared document model had no milestone -- `MILE-4` was marked absorbed into `RFC-33` and the work moved into an RFC, so six records were blocked on something the plan did not track. Naming it makes the dependency resolvable, and `narrative-field.stale` can report when it moves. | **structural** |
> | 2026-11-04 | Built as the opt-in rule `section.shape` with shape `y-statement` on a required section (`SPEC-3`). `Status` stays `Planned`. **Why:** the shape is a declared parameter of `sections.required`, per `ADR-53`. The five phrases are fixed, not declarable, because this milestone names them and nothing asks for others; they are matched on word boundaries, case-insensitively, over one block. An optional `marker` opens the block at the first line containing it (a declared marker the section lacks, and a marker with no block after it, are each their own finding); with no marker the section's first paragraph is the block. A list item or blank line ends the block. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged on `main`: opt-in rule `section.shape` with shape `y-statement` checks the five phrases on word boundaries and reports a marker with no block. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | `Blocked-on` now reads `MILE-98`. **Why:** `narrative-field.stale` flagged a blocker that reached `Done`, and the delivery backfill of `MILE-183` set it; the remaining blocker is still open. `Status` unchanged. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-04 | `Blocked-on` cleared: `MILE-98` is Done (declared `identity`, `fields` and `sections`, including `###` headings read through `sections.depth`). `Status` unchanged. **Why:** `narrative-field.stale` flagged a blocker that reached a terminal status; the dependency no longer holds. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
