---
Stable-Id: 01M373DY8GRKTBHPMVVDJAFGSE
Status: Fixed
Found-in: "MILE-112's duplication sweep"
Regression-test: "filename_title_consistency_reads_the_number_past_a_hyphenated_prefix, rules.rs"
---
# 139 — record_id and filename_number independently re-implement the same filename-id extraction

## What was wrong

`record_id` (`rules.rs`) and `filename_number` (`rules.rs`) were byte-for-byte identical except their
last line: both strip the type prefix off a filename stem, split at the first hyphen to isolate the
number, and validate it's non-empty and all-digit. `record_id` returns `"{prefix}-{number}"`;
`filename_number` returns just `number.to_string()`. `filename_number`'s own doc comment said *"Same
acceptance as `record_id`"* -- acknowledged, never shared.

Exactly `BUG-133`'s shape one level over: not the segment predicates inside the grammar, but the
id-from-filename extraction itself.

## Why nothing caught it

Two independently hand-written copies of one filename convention, with nothing enforcing they agree.
Both currently accept and reject the same filenames, so there was no live divergence to trip a test --
only the same latent risk `BUG-111`/`BUG-114` already realized twice for the reference/filename segment
predicates this sits beside.

## Fix

`filename_number` now calls `record_id` and takes the number half of its `TYPE-NNNN` result via
`rsplit_once('-')` -- correct even for a hyphenated type prefix (`DOC-ADR-2`), since the number segment
is always last and all-digit by `record_id`'s own guard. No behavior change for the existing shape; a
new regression test locks in the hyphenated-prefix case explicitly, since nothing exercised it for
`filename_number` before.

## References

- `record_id` -- the extraction `filename_number` now defers to.
- `BUG-133` -- the same duplication-drift risk, one level lower (the segment grammar inside filename
  parsing, not the filename-id extraction as a whole).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass, found by MILE-112's duplication sweep. **Why:** two independently hand-written copies of the same filename-id extraction, one level over the exact pattern `BUG-133` already fixed. No live divergence yet, but past `SPEC-22`'s second-occurrence threshold. | **substantive** |
