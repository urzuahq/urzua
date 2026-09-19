---
Stable-Id: 01M2W32794T1K6A3YCY3KE2XXQ
Status: Fixed
Found-in: 'A cumulative code review of v0.3.0..release, run before publishing 0.4.0 -- reproduced against the built binary'
Regression-test: 'not yet written -- a corpus holding a date-named file alongside `ADR-1-x.md` must yield 2, not 2027'
---
# 53 — A date-named file hijacks `urzua new`'s numbering permanently

## What is wrong

`parse_record_filename`'s bare-number branch accepts **any** all-digit first segment with no length
constraint. `init`'s previous recogniser required exactly four digits; `BUG-37` made both halves share
this one, and dropped the constraint in the process.

`next_display_number` delegates to it, so a single date-named file poisons the sequence. Reproduced
against the built binary:

```text
docs/adr/ADR-1-x.md
docs/adr/2026-09-19-meeting-notes.md

$ urzua new adr "Second one"
display_number: 2027   path: docs/adr/ADR-2027-second-one.md
```

**Numbers are never reused**, so that corpus is stuck above 2027 permanently. The pre-`BUG-37` code
returned 10 on the same fixture.

## Why it matters more than the bug it came from

`BUG-37` fixed a real defect -- `new` writing a duplicate number into an adopted corpus -- and
introduced a worse one. A duplicate number is visible and correctable; a sequence jumped to 2027 is
neither, and a date-named file in a records directory is ordinary rather than exotic.

## Fix

Constrain the bare-number branch. One to four digits keeps every corpus this recogniser was widened
for -- Nygard's `0001`-`9999`, this project's own history -- while a `2026-09-19` prefix stops
parsing as a record number. Four digits is not arbitrary: it is what `init` required before
`BUG-37`, and no observed corpus numbers past it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed, blocking the 0.4.0 release. **Why:** found by a cumulative review of the release diff, which is the first review of the shipped tree as a whole rather than of the PRs that built it. `BUG-37` shared one recogniser between `init` and `new` and dropped the four-digit constraint doing so; the corruption it causes is permanent because numbers are never reused. | **substantive** |
> | 2026-09-19 | `Status: Open` → `Fixed`. **Why:** the bare-number branch now rejects a date tail -- `NN-NN-` after the first segment is a date, not a slug. A four-digit bound does not discriminate, because a year is four digits, which the first attempt at this fix got wrong. Verified: the fixture returns 2 where it returned 2027. Observed failing. | **substantive** |
