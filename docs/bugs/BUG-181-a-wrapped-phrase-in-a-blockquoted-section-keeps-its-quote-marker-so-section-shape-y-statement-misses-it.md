---
Stable-Id: 01M6JKNX8RP2B6G73ZDAAFGPCQ
Status: Fixed
Found-in: 'a trial run over a real adopter corpus (2026-11-04)'
Regression-test: '`a_phrase_wrapped_across_blockquote_lines_is_found` in `rust/crates/urzua-core/src/rules/section.rs`'
---
# 181 — A wrapped phrase in a blockquoted section keeps its quote marker so section.shape y-statement misses it

## What was wrong

In `section.shape` with the `y-statement` shape, continuation lines of a blockquoted block keep their leading `> `, so a phrase that wraps across lines (`to` at the end of one line, `achieve` at the start of the next) is not found. A real adopter corpus of several hundred records produced dozens of false errors, all of which disappear when a leading `>` is stripped from each continuation line before matching.

## Why nothing caught it

No test wrapped a required phrase across lines of a blockquote; the fixtures kept each phrase on one line, so the quote marker never sat between two words of a phrase.

## References

BUG-1 of the adopter switchover report. Fix in `rust/crates/urzua-core/src/rules/section.rs` `y_statement`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this defect. | **substantive** |
> | 2026-11-04 | Fixed. `y_statement` strips a leading `>` from each line before matching (`SPEC-3` 0.27). **Why:** the quote marker no longer sits between two words of a phrase. | **substantive** |
> | 2026-11-04 | References corrected: the fix location is now the full repository path of `section.rs`, which `citation.path-exists` (`MILE-143`) could not find under its shortened form. `Status` is untouched. **Why:** the rule is now enabled in this repository. | **structural** |
> | 2026-11-04 | `Regression-test` filled in: the real value. `Status` is untouched. **Why:** `field.pending` now reads a `Regression-test` placeholder on a `Fixed` bug as a real gap (`MILE-169`), and the `Regression-test` field named a test that was not yet written; it now names the test that wraps a required phrase across blockquote lines. | **structural** |
