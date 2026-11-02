---
Stable-Id: 01M6D2KGHR61YWA0TD4338QWZ1
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 29"
Regression-test: "fix_apply_appends_inside_a_plain_revision_log_table (rust/crates/urzua-cli/tests/check_integration.rs); apply_repair_appends_a_plain_table_row_inside_the_plain_table_observed_failing (rust/crates/urzua-core/src/fix.rs)"
---
# 165 — fix --apply appends its revision-log row with a hardcoded blockquote prefix, detaching it from a plain table

## What was wrong

`apply_repair` built its revision-log row as `format!("> | {today} | ...")`, assuming the log is a
blockquote table. `find_revision_log_entries` accepts both forms (it strips a leading `>`), so a record
whose log is a plain table
(`**Revision log**` / `| Date | Change | Class |` / `|---|---|---|` / `| 2026-01-01 | ... |`) was
accepted for repair, and the new row came out as a blockquote line after a plain table. In rendered
Markdown it sits outside the table. `urzua check` still reported ok, since the same tolerant parser
reads it as a row.

## Why nothing caught it

The test used a blockquote log, matching the project's own records. The parser's tolerance for both
forms was never paired with a writer test for the second one.

## Fix

The new row copies the prefix of the existing last row (everything before its first `|`), so it stays in
the same table in either form. Done through the shared `LineDoc` helper (`BUG-164`), which also
preserves line endings.

## References

- `BUG-164` -- the whole-file rebuild this is a symptom of.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed and fixed in the same change. **Why:** found by a full-release code review and reproduced with the release binary on a disposable fixture; the regression test was observed failing before the fix and passing after. | **substantive** |
