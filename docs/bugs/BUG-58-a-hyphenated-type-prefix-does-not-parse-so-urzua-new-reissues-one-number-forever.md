---
Stable-Id: 01M2WA9W3CBVRXXBCH2VTFTS6T
Status: Fixed
Found-in: 'The fifth cumulative review of the 0.4.0 release diff, reproduced end to end against the built binary'
Regression-test: 'rust/crates/urzua-core/src/new_record.rs::a_hyphenated_type_prefix_parses_observed_failing -- DOC-ADR-2-slug parses, a lower-case segment does not, a number with nothing after it does not, and next_display_number advances'
Blocked-on: —
---
# 58 — A hyphenated type prefix does not parse, so `urzua new` reissues one number forever

## What was wrong

`parse_record_filename` took the **first** `-`-segment as the type prefix, so a prefix containing a
hyphen never parsed. `init` emits exactly that when two directories share a last component -- `doc/adr`
and `docs/adr` both want the name `adr`, so they are qualified to `doc-adr` and `docs-adr` -- and `new`
derives the filename prefix from the type name.

Reproduced end to end:

```text
$ urzua new doc-adr "First thing"    ->  doc/adr/DOC-ADR-2-first-thing.md
$ urzua new doc-adr "Second thing"   ->  doc/adr/DOC-ADR-2-second-thing.md
```

The same number, forever: `next_display_number` cannot read back the files `new` itself wrote, so it
sees an empty directory every time. Duplicate display numbers are the failure `BUG-37` records as
unrecoverable, reached through a path added while fixing `BUG-36`.

## Why nothing caught it

Both halves were tested against the shape the author had in mind. `parse_record_filename`'s tests
covered `ADR-1-slug` and `0001-slug`; `init`'s collision-qualification test asserted the *name* it
produced and never fed that name back to `new`. The two functions agree about `ADR`, disagree about
`DOC-ADR`, and nothing exercised the round trip.

## Fix (shipped)

The number is **found**, not assumed to be the second segment: the first all-digit segment is the
number, everything before it is the prefix, and every prefix segment must be upper-case. `DOC-ADR-2-x`
parses; `doc-ADR-2-x` does not; `DOC-ADR-2` does not, being a fragment with nothing after the number.

Shipped alongside a narrowing of the date guard (`BUG-53`), which had rejected any bare-number filename
whose slug began with two 2-digit segments: `0013-80-20-rule.md` is a record, and only a four-digit
year with a month of 1-12 and a day of 1-31 is a date.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed and fixed. **Why:** found by the fifth cumulative review of the release diff. Two functions agreed about the filename shape the author had in mind and disagreed about the one the tool generates, with nothing exercising the round trip between them. | **substantive** |
