---
Stable-Id: 01M33QPM2CV2ZD54KTF9YXBWXR
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, reviewing everything landed for the 0.4.0 release"
Regression-test: "a_date_named_file_is_not_a_record_number_observed_failing (extended case), new_record.rs"
---
# 102 — new record's date heuristic can reissue a colliding record number

## What was wrong

`parse_record_filename`'s `looks_dated` heuristic (`new_record.rs`) treats a filename as an unnumbered,
date-stamped file — and excludes it from `next_display_number`'s max — whenever its first three
hyphen-separated segments are 4-digit, then a plausible month (1-12), then a plausible day (1-31).
`0013-01-15-use-postgres.md` satisfies all three checks (`"0013"` is 4 digits, `"01"` is a plausible
month, `"15"` is a plausible day) and was treated as an unnumbered date, even though it is record `13`
with a slug that happens to start with two date-shaped segments. `next_display_number` then skips it
when computing the corpus's highest number, so `urzua new` could reissue `13` and collide with the file
that already claims it — reintroducing the duplicate-numbering defect (`MILE-51`/`BUG-37`) this
heuristic exists to avoid triggering falsely on.

## Why nothing caught it

The existing regression test for this heuristic (`a_date_named_file_is_not_a_record_number_observed_failing`)
covered `0013-80-20-rule.md`, which escapes the heuristic via the month check (`80` is not `1..=12`), and
never exercised a genuinely ambiguous case where the record number's own digits happen to look like a
valid month and day.

## What changed

`looks_dated` now also requires the leading segment's parsed value to be a plausible year
(`1000..=9999`, checked as a number, not just as a 4-character string). `"0013"` parses to `13`, which
fails that bound, so the filename is correctly read as record `13`. A genuine dated filename
(`2026-09-19-...`, `1999-01-01-...`) is unaffected: both parse well inside the bound.

## References

- `MILE-51`, `BUG-37` — the duplicate-numbering defect this heuristic exists to avoid re-triggering.
- `BUG-9` — the original decision to exclude the unnumbered legacy filename shape.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-22 | Filed and fixed in one pass. **Why:** found by a full-release code review; a bounds check added to an existing heuristic, verified with a planted-violation test observed failing before the fix. | **substantive** |
