---
Stable-Id: 01M36S8ZRSNACZH1K3JKRPY02P
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 24"
Regression-test: a_recognized_reference_parses_as_the_same_filename_prefix (rust/crates/urzua-core/src/rules.rs)
---
# 133 — is_record_reference and parse_record_filename independently re-implement the same record-identifier grammar

## What was wrong

`is_record_reference` (`rules.rs`, scans prose/pointer-field values for record mentions) and
`parse_record_filename` (`new_record.rs`, parses a filename into prefix + number) both implement "is
this a valid record identifier" -- hyphen-segment splitting, an all-digit number segment, uppercase
(and, since `BUG-114`, digit-bearing) prefix segments. `is_record_reference`'s own comment already
says the two "must agree," but nothing enforces it.

`BUG-114`/round 23 already found and fixed one instance of these two drifting apart (a digit-bearing
prefix). The underlying duplication that let that drift happen in the first place is still there.

## Why this needs a decision, not a quick extraction

The two functions don't operate on the same input shape: `parse_record_filename` handles a filename
stem specifically (date-heuristic exclusion, ambiguous-prefix-vs-number resolution via `at`'s
first-digit-segment scan, ADR-36's legacy-shape rejection), while `is_record_reference` handles a bare
reference token with no filename-specific context. A literal shared function would either need to
absorb `parse_record_filename`'s filename-specific logic into the reference recognizer (wrong -- a
reference token is not a filename) or extract only the common prefix/number grammar into a helper both
call, changing both functions' structure. Worth doing, not obviously a small edit.

## Fix

The two functions' overall shapes stayed separate, as the filing anticipated -- `parse_record_filename`
still does its own filename-specific work (date-heuristic exclusion, the first-digit-segment scan for
`at`), and `is_record_reference` still does its own token-scanning work. What's shared is narrower than
"the grammar" as a whole: exactly the two boolean segment predicates both independently re-implemented
byte-for-byte identically -- "is this a valid number segment" and "is this a valid prefix segment".
Extracted as `new_record::is_digit_segment`/`is_prefix_segment`, `pub(crate)`, both functions now call
the same code instead of two copies that can silently diverge (as they already had, twice: `BUG-111`,
`BUG-114`). No behavior change; a regression test pins the two recognizers' agreement on a shared set of
tokens so a future edit to one predicate cannot re-open the drift without failing a test that names both
call sites.

## References

- `BUG-114` -- the specific divergence already found and fixed once; this bug is the general
  duplication that produced it.
- `BUG-111` -- the earlier hyphenated-prefix instance of the same two recognizers disagreeing.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed. **Why:** found by a full-release code review, immediately after fixing a live instance of the exact drift this duplication produces (`BUG-114`). Left `Status: Open` -- extracting a shared grammar needs deciding its shape, not a quick copy-paste fix. | **substantive** |
> | 2026-09-23 | Fixed. Narrower than the filing anticipated: only the two boolean segment predicates were byte-for-byte duplicated, not the surrounding filename/token-specific logic each function keeps. Extracted `is_digit_segment`/`is_prefix_segment` into `new_record.rs`; `is_record_reference` now calls them. Regression test pins the two recognizers' agreement. | **substantive** |
