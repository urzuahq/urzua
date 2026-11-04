---
Stable-Id: 01M6JKNZ1F635Y5F0NEPWB3FWC
Status: Fixed
Found-in: 'a trial run over a real adopter corpus (2026-11-04)'
Regression-test: '`a_bold_list_header_is_named_as_an_unsupported_form` in `rust/crates/urzua-core/src/header.rs` and `a_removed_header_shape_is_a_clear_error_naming_the_form` in `rust/crates/urzua-core/src/config.rs`'
---
# 184 — The bold-list header reader ends the header at a wrapped value

## What was wrong

The bold-list header reader ends the header at a wrapped value (an indented continuation line), so every field after it reads as absent. On a real adopter corpus not yet migrated this produced over a hundred false `status.required-field-missing` findings on one type and a dozen more on another, which makes an as-is run untrustworthy. Full removal of the blockquote and bold-list readers (`MILE-85`) resolves it; it is recorded so the interim state is not mistaken for a real finding.

## Why nothing caught it

Fixtures kept every bold-list field on one line, so no test wrapped a value.

## References

BUG-4 of the adopter switchover report. Resolved by `MILE-85`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this defect. | **substantive** |
> | 2026-11-04 | Fixed by removing the bold-list reader (`MILE-85`): a bold-list header is no longer read, so there is no wrapped-value reader to end early. A config declaring `bold-list` fails to load and a record written in it is reported unreadable with the form named, so the interim false `status.required-field-missing` findings cannot recur. **Why:** the maintainer's full-cleanup decision on `MILE-85`. | **substantive** |
> | 2026-11-04 | `Regression-test` filled in: the real value. `Status` is untouched. **Why:** `field.pending` now reads a `Regression-test` placeholder on a `Fixed` bug as a real gap (`MILE-169`), and the `Regression-test` field named a test that was not yet written; the defect was resolved by removing the bold-list reader (`MILE-85`), and the tests that now pin the removal are named. | **structural** |
