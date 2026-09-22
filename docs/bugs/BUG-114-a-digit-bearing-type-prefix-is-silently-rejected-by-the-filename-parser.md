---
Stable-Id: 01M34QAPNDG4YZVQDZVQKVMSZY
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, reviewing everything landed for the 0.4.0 release"
Regression-test: "a_hyphenated_type_prefix_parses_observed_failing, new_record.rs"
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
