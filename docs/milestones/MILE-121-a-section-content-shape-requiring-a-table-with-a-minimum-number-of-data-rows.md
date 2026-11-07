---
Stable-Id: 01M6CXZW5CJV2350GRPQ4XZRXK
Status: Done
Phase: '0'
Track: section-checks
Implements: RFC-17
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/section.rs#table_min_rows@sha256:320fb11f90d17a8b1e7dc04bba97433893ac4b303b85fd09e6d9d1fdf3c0460a, test:rust/crates/urzua-core/src/rules/section.rs#the_larger_of_two_tables_decides@sha256:baca77d72e33796884018971b1a0653d78a1781fbac1749c33ee0210bf3b8de4, test:rust/crates/urzua-core/src/rules/section.rs#an_all_empty_row_is_not_a_data_row@sha256:64b9106f2471f0de8fafb9ce9bde5cebba2bc54f921d5de02508192e9310a926, code:rust/crates/urzua-core/src/rules/section.rs#section_shape@sha256:d0695c8069a1661f52c5d48ec05dba41f7697e1fc5fa3f3e11e1322c6484c0bc, test:rust/crates/urzua-core/src/rules/section.rs#one_data_row_fails_and_says_how_many@sha256:227d8033e057a74b3bb9f254101057afbbeebe5f569b8bb687e5268ca43c66f9, test:rust/crates/urzua-core/src/rules/section.rs#a_header_only_table_and_no_table_are_both_findings@sha256:dc4c3181db50e0630fd5904f2e78cf7459c3b96190c241ef8002bd60a01a6877, test:rust/crates/urzua-cli/tests/section_rules_integration.rs#declared_rules_report_each_failure_and_count_slots@sha256:fd72db5c46d6b283f04a776dfb01e5fec3d7b8e2e28a7a6dcf2fac68e22c5376, spec:docs/specs/SPEC-3-configuration.md
---
# 121 — A section content shape requiring a table with a minimum number of data rows

## What

A section content shape that requires a section to contain a table with at least N data rows, for
example a considered-options table with at least two options. `RFC-17` names it as the plausible
second shape after `y-statement` and leaves it undesigned. Build it on the section parser from
`MILE-98` (the parser moved there from `MILE-4` on 2026-09-19), declared in config like any other shape.

## Why

A decision record whose options table has one row has not considered an alternative, and presence of the
section does not show that. Replacing a hand-written linter outright (`MILE-102`) means the content shapes it checks, a
row-count minimum among them, need declared equivalents. The rule checks shape (row count), never
whether the options are real, per `AGENTS.md` ("Structural presence is not content-scope correctness").

## Done means

- A required section can declare the shape `table-min-rows` with `min_rows`, counting the data rows of the largest table in the section. — evidence: code:rust/crates/urzua-core/src/rules/section.rs#table_min_rows@sha256:320fb11f90d17a8b1e7dc04bba97433893ac4b303b85fd09e6d9d1fdf3c0460a, test:rust/crates/urzua-core/src/rules/section.rs#the_larger_of_two_tables_decides@sha256:baca77d72e33796884018971b1a0653d78a1781fbac1749c33ee0210bf3b8de4, test:rust/crates/urzua-core/src/rules/section.rs#an_all_empty_row_is_not_a_data_row@sha256:64b9106f2471f0de8fafb9ce9bde5cebba2bc54f921d5de02508192e9310a926
- `section.shape` reports a section with fewer data rows, with no table, or with only a header row, and a fixture covers each. — evidence: code:rust/crates/urzua-core/src/rules/section.rs#section_shape@sha256:d0695c8069a1661f52c5d48ec05dba41f7697e1fc5fa3f3e11e1322c6484c0bc, test:rust/crates/urzua-core/src/rules/section.rs#one_data_row_fails_and_says_how_many@sha256:227d8033e057a74b3bb9f254101057afbbeebe5f569b8bb687e5268ca43c66f9, test:rust/crates/urzua-core/src/rules/section.rs#a_header_only_table_and_no_table_are_both_findings@sha256:dc4c3181db50e0630fd5904f2e78cf7459c3b96190c241ef8002bd60a01a6877, test:rust/crates/urzua-cli/tests/section_rules_integration.rs#declared_rules_report_each_failure_and_count_slots@sha256:fd72db5c46d6b283f04a776dfb01e5fec3d7b8e2e28a7a6dcf2fac68e22c5376
- `SPEC-3` documents the shape. — evidence: spec:docs/specs/SPEC-3-configuration.md

## References

- `RFC-17` -- composable section-content checks; names `table-min-rows` and leaves it undesigned.
- `MILE-4` -- the section parser.
- `MILE-102` -- the adopter replacement this serves.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** `RFC-17` names `table-min-rows` as a plausible second shape; a hand-written linter's row-count check needs a declared equivalent, so it is now planned. | **substantive** |
> | 2026-11-04 | Built as shape `table-min-rows` on a required section, with `min_rows` declared per section (`SPEC-3`), checked by `section.shape`. `Status` stays `Planned`. **Why:** a considered-options table with one row has considered no alternative. The shape counts data rows of the largest table in the section, ignoring fenced code and rows whose every cell is empty; it checks row count only, never whether the options are real. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged on `main`: shape `table-min-rows` with `min_rows` declared per section, checked by `section.shape`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-04 | `Blocked-on` cleared: `MILE-98` is Done (declared `identity`, `fields` and `sections`, including `###` headings read through `sections.depth`). `Status` unchanged. **Why:** `narrative-field.stale` flagged a blocker that reached a terminal status; the dependency no longer holds. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
