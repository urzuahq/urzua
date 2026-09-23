---
Stable-Id: 01M36WECY7ZHHCHDCCZTJQ1TAT
Status: Planned
Phase: '1'
Track: header-format
Implements: ADR-57
---
# 113 — an opt-in rule flagging two keys in one record differing only by case, composable alongside exact-match duplicate detection

## What

A new, opt-in rule (working name `header.case-variant-key`) that reports when one record's own
header writes two keys differing only by case (`Status`/`status`) -- a distinct signal from
`duplicate_keys()`'s exact-match repetition check, not a replacement for it. `ADR-57`'s exact-match
principle stays exactly as decided: this rule never asserts which spelling is "right," never treats
the two keys as one field, and never affects `duplicate_keys()`, `header.field-set-consistency`, or
any other exact-match comparison. It only surfaces the fact of the case-only difference as its own,
separately-enable-able finding, for a repository that wants that extra scrutiny.

## Why

Came out of investigating `BUG-130`: a real, live gap exists (a type declaring no vocabulary has
nothing that flags a `Status`/`status` pair in one header as suspicious), but the fix isn't to make
`duplicate_keys()` case-insensitive -- `ADR-57` already, explicitly, decided that comparison stays
exact, for the same reason it stays exact everywhere else (measured: zero legitimate case-only field
pairs in this corpus's own history; `Maße`/`Masse` is exactly the shape a loose comparison gets wrong
in the other direction). The right shape is a new, additive, opt-in signal composed alongside the
existing exact-match rules, not a change to any of them -- the same pattern `header.field-case-
mismatch` (`RFC-40`/`ADR-58`) already uses for a declared-field case mismatch, generalized to a
same-record, no-declaration-required case.

## References

- `BUG-130` -- the investigation that surfaced this, corrected to `Not a bug` rather than folded
  into this milestone's own scope.
- `ADR-57` -- the exact-match principle this milestone does not reopen or weaken.
- `header.field-case-mismatch` (`RFC-40`/`ADR-58`) -- the closest existing precedent for a
  case-focused, additive signal that doesn't touch exact-match verdicts.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed. **Why:** the user proposed this as a genuinely separate, composable rule while working through `BUG-130` with me -- filed as its own milestone rather than bundled into that bug's correction, since it's new work, not a fix to a misdiagnosed one. | **substantive** |
