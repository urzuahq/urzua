---
Stable-Id: 01M34QAPNDG4YZVQDZVQKVMSZY
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, reviewing everything landed for the 0.4.0 release"
Realized-by: test:rust/crates/urzua-core/src/new_record.rs#a_hyphenated_type_prefix_parses_observed_failing@sha256:6c70397f7b9475d1a181aa9eb1064ce83842f96ecd3017c58ca0fdae478b57a5
---
# 114 — a digit-bearing type prefix is silently rejected by the filename parser

## What was wrong

`parse_record_filename` required every prefix segment to be pure ASCII-uppercase letters.
`RecordTypeConfig.prefix` places no such restriction on what an adopter can configure. A type
declaring a prefix like `V2` or `S3`, with files like `V2-3-migrate.md`, would never parse: the
filename fails the prefix check and returns `None`, so `next_display_number` never sees the existing
number and can hand out an already-used one to `urzua new` — the same duplicate-numbering failure
class `BUG-37`/`MILE-51` already fixed for a different cause.

## Why nothing caught it

Every existing test used a pure-letter prefix (`ADR`, `RFC`, `DOC-ADR`); nothing exercised a
digit-bearing one, and nothing in `config.rs` validates `prefix` against the shape
`parse_record_filename` actually expects.

## What changed

The prefix-segment check now allows digits alongside uppercase letters. This is safe without
introducing ambiguity: `at` (the number segment's position) is already the *first* all-digit segment,
so any segment before it is guaranteed to contain at least one non-digit character — a mixed
alphanumeric prefix can never be confused with the number segment.

## References

- `BUG-37`, `MILE-51` — the same duplicate-numbering failure class, different cause.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass. **Why:** found by a full-release code review; the existing hyphenated-prefix test already covered the right shape, extended with a digit-bearing case. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/new_record.rs#a_hyphenated_type_prefix_parses_observed_failing` (lines 532-561) 7fb83ce76730 -> 6c70397f7b94. Reason: Read the diff of each pinned span since the old pin and the claim still holds. new_record.rs#a_hyphenated_type_prefix_parses_observed_failing: the calls to next_display_number now unwrap its Result (it reports an exhausted number instead of overflowing), the expected numbers are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
