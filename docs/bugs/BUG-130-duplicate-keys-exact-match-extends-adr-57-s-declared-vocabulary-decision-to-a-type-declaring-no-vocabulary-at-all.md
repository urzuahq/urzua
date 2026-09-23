---
Stable-Id: 01M36PD8JJ4SF9KQ53EYYET1V2
Status: Not a bug
Found-in: "A /code-review v0.3.0...main pass, round 23"
Regression-test: "none -- see Amendment"
---
# 130 — duplicate_keys exact-match extends ADR-57's declared-vocabulary decision to a type declaring no vocabulary at all

## What was suspected

`Header::duplicate_keys()` compares field keys exactly. For a type with no `known_fields`/
`required_fields` declared at all, nothing else catches a `Status`/`status` repetition either
(`header.field-set-consistency` and `header.required-fields`'s own duplicate-key check both only
examine records of a type present in their respective declared-field maps). The reviewer read
`duplicate_keys()`'s exact-match behavior as an unexamined extension of `ADR-57` -- a decision whose
own stated context is declared-vocabulary matching, not within-document duplicate detection -- into a
different concern it never actually addressed.

## Why the diagnosis was wrong

`ADR-57`'s own Decision section names `duplicate_keys` explicitly, by name, in the same sentence as
`Header::get` and `discovery`'s `Realized-by` lookup:

> "This applies to every comparison of a field *name*, not to one rule. `Header::get`... `duplicate_keys`
> and `discovery`'s `Realized-by` lookup had the same split. All are exact."

`duplicate_keys()`'s exact-match behavior is not an unexamined consequence of applying "field names
compare exactly" broadly -- it is one of the three specific comparisons `ADR-57` was written to fix,
in the same pass, for the same reason (`Header::get` and `header.field-set-consistency` used to
disagree about one field because of exactly this kind of split). Filing this as needing a fresh
decision was itself the error this project's "verify before trusting" discipline exists to catch: the
finding never re-read `ADR-57`'s own text closely enough to find the sentence that already answers it.

## What changed

Nothing in the code. `duplicate_keys()`'s exact-match comparison is correct as written, per an
existing, evidence-backed, `Accepted` decision that already covers it by name.

A separate, genuinely new idea came out of this investigation: a distinct, opt-in rule that flags two
keys in the *same record* differing only by case as their own signal (composable alongside, not a
replacement for, exact-match duplicate detection) -- filed as `MILE-113` rather than folded into this
correction, since it's new work, not a fix to a misdiagnosed bug.

## References

- `ADR-57` -- names `duplicate_keys` explicitly as one of the three comparisons this decision fixed.
- `BUG-127` -- the precedent for this correction's shape: investigated, found unreachable/incorrect as
  diagnosed, re-graded rather than silently deleted.
- `MILE-113` -- the new, separate rule idea this investigation surfaced.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed, `Status: Open`. **Why:** found by a full-release code review; verified the gap is real for a type declaring neither `known_fields` nor `required_fields`. | **substantive** |
> | 2026-09-23 | Amended same day: re-read `ADR-57`'s own Decision section in full while working through this bug with the user, found it explicitly names `duplicate_keys` as one of the three comparisons the decision was written to fix -- directly contradicting this record's own "ADR-57 itself... doesn't mention `duplicate_keys()`" claim. Re-graded `Status: Not a bug`. **Why:** this project's own "never silently rewrite" rule -- correcting the record in place, in the open, rather than deleting it. | **substantive** |
