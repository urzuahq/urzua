---
Stable-Id: 01M34MYQ7YW4RZH9GDS01AGS1S
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, reviewing everything landed for the 0.4.0 release"
Realized-by: test:rust/crates/urzua-core/src/new_record.rs#a_date_named_file_is_not_a_record_number_observed_failing@sha256:ff7cab7405b7c38e8e47cacd78e62045a6e3e031d4eb110734f14fe6e6e391d2
---
# 112 — the date filename heuristic requires zero-padded month and day segments

## What was wrong

`parse_record_filename`'s `looks_dated` heuristic required the month and day segments to be exactly
two characters (`segments[i].len() == 2`). `2026-9-19-meeting-notes.md` — a genuine date, just
unpadded — failed that length check, so `looks_dated` was `false` and the filename parsed as record
`2026`, the same symptom `BUG-53`/`BUG-102` already cover for the padded shape.

## Why nothing caught it

The existing regression tests for this heuristic only used zero-padded dates
(`2026-09-19-meeting-notes.md`, `1999-01-01-x.md`); nothing exercised an unpadded one.

## What changed

The length-2 requirement is dropped; only the numeric value range (`1..=12` for month, `1..=31` for
day) is checked, which already rejects anything that isn't a plausible month or day regardless of how
many digits it has. `0013-80-20-rule.md` (an ADR titled "80-20 rule") is unaffected: `80` still fails
the month range regardless of length.

## References

- `BUG-53`, `BUG-102` — the same defect shape for the padded date case.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-22 | Filed and fixed in one pass. **Why:** found by a full-release code review; a planted-violation test observed failing against the length-2 requirement before the fix. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/new_record.rs#a_date_named_file_is_not_a_record_number_observed_failing` (lines 563-601) 6ca017966198 -> ff7cab7405b7. Reason: Read the diff of each pinned span since the old pin and the claim still holds. new_record.rs#a_date_named_file_is_not_a_record_number_observed_failing: the calls to next_display_number now unwrap its Result (it reports an exhausted number instead of overflowing), the expected numbers are unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
