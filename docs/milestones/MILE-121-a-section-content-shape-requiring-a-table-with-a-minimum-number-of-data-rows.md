---
Stable-Id: 01M6CXZW5CJV2350GRPQ4XZRXK
Status: Done
Phase: '0'
Track: section-checks
Implements: RFC-17
Blocked-on: MILE-98 (the section parser this shape is built on)
Realized-by: code:rust/crates/urzua-core/src/rules/section.rs, test:rust/crates/urzua-cli/tests/section_rules_integration.rs
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
