---
Stable-Id: 01M30ME8D4DNR4TDBSZPDPVVBY
Status: Open
Found-in: "Reading the ten `to_ascii_lowercase` call sites while planning the property suite (MILE-101); confirmed against the Rust standard library rather than against the engine"
Regression-test: "not yet written -- a type declaring a field name with a non-ASCII letter, and a record writing that same name, must not report the field as undeclared"
Blocked-on: MILE-101
---
# 97 — a field name with a non-ascii letter is compared by ascii folding so a declared field reports undeclared

## What was wrong

Field names are compared case-insensitively via `to_ascii_lowercase`, which lowercases `A-Z` and
leaves every other byte alone. So `"CAFÉ".to_ascii_lowercase()` is `"cafÉ"`, while a config declaring
`café` lowercases to `café`. The two never match.

The consequence is a false finding rather than a missed one: `header.field-set-consistency` reports a
field the type *does* declare as not declared, and there is no way for the adopter to spell the
declaration so that it matches. Any corpus whose field vocabulary is not pure ASCII is affected, which
is the corpus family `MILE-51` exists to adopt.

Ten call sites: six in `rules.rs`, one each in `header.rs`, `field_state.rs`, `new_record.rs` and
`check.rs`.

The fix is a decision, not a substitution. Full Unicode case folding is correct but locale-dependent
at the edges (`İ` lowercases to `i̇`, two code points), and an ASCII-only comparison that *reports*
its own limit is a defensible alternative. It wants an `ADR`.

## Why nothing caught it

Every fixture in the suite spells its field names in ASCII, so the comparison has never been given an
input that could distinguish the two behaviours. `BUG-87` established that the corpus contains
accented *filenames*; nothing extended that to field names. This is the class of gap the property
suite (`MILE-101`) exists to close, and it is filed ahead of that suite specifically so that the suite
finding it counts as the suite working rather than as a surprise.

## References

- `MILE-101`, the property suite whose curated alphabet includes `é É ß İ`.
- `BUG-87`, the accented-filename case.
- `ADR-53` -- a field vocabulary is the adopter's declaration, so the engine must be able to compare
  whatever they declare.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-21 | Filed. **Why:** found by reading the call sites while planning `MILE-101`, and confirmed directly rather than by inspection. Filed before the suite that will find it, so that the suite's first real finding is a prediction met rather than a new discovery. | **substantive** |
