---
Stable-Id: 01M2WDKAQJMAQYP0TD4ZN02W2Z
Status: Fixed
Found-in: "A reviewer noticed five records claiming no regression test existed while the same branch added them; field.pending had examined every one of those fields and passed them"
Regression-test: "rust/crates/urzua-core/src/rules/field.rs::a_declared_pending_vocabulary_matches_whole_values_observed_failing"
Blocked-on: —
---
# 65 — field.pending's vocabulary is compiled in so the corpus's own commonest way of saying pending reads as a real value

## What was wrong

`classify` decides a field is `Pending` when its value contains `pending` or equals `todo`. Those two
tokens are compiled into `field_state.rs`.

This corpus says the same thing a different way. **19 of 64 bug records** write
`Regression-test: "not yet written -- ..."`, which contains neither token, so `classify` falls through
to `Present`: a real value, nothing to report.

The measurement: `field.pending` examined **928** fields across the corpus and produced **3** findings.
Five of the records it passed in that run were the round-6 bugs on this branch, each stating no
regression test existed while the same branch added one.

The rule read the field, reached a verdict, and the verdict was wrong -- the vocabulary it judges
against is not the vocabulary the corpus uses, and an adopter's will differ again. `ADR-53` decided
every rule is a declared policy; which phrases mean "not done yet" is policy, and it is not
declarable.

## Why nothing caught it

`field.pending` has tests, and they pass, because they were written against the two tokens the
function already knew. A test that supplies the vocabulary the implementation defines can only ever
confirm it.

Nothing compared the token list against the corpus the rule runs on. That comparison is a one-line
grep and it had never been run.

## References

- `ADR-53` -- governance is configuration; every rule is a declared policy.
- `BUG-59`, the same defect in the status vocabulary, blocked on `MILE-98`. The pending vocabulary is
  the same question about a different field, and the two should be decided together.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | A type now declares its own pending values beside its status vocabulary (`statuses.pending`, `SPEC-3`), matched as whole values and replacing the compiled-in token for that type; a type declaring none keeps today's match. `Regression-test` now names the test. `Status` unchanged. **Why:** maintainer decision of 2026-11-04 (`MILE-132`); the substring match was the defect, so this repository's `BUG-38` and `BUG-65` values, which mention the word, no longer read as pending. | **substantive** |
> | 2026-11-04 | `Status` set to `Fixed`; `Blocked-on` cleared. Fixed in #173 (`MILE-132`): a type declares its pending values beside its status vocabulary, matched as whole values, and this repository's `bug` type declares `not yet written` and `planned:`; `Regression-test` already names the test. **Why:** the compiled-in vocabulary was the defect and the declaration replaces it, so the record's `MILE-98` dependency no longer holds. | **substantive** |
