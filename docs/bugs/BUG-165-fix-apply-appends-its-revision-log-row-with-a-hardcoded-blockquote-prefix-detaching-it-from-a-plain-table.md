---
Stable-Id: 01M6D2KGHR61YWA0TD4338QWZ1
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 29"
Realized-by: test:rust/crates/urzua-cli/tests/check_integration.rs#fix_apply_appends_inside_a_plain_revision_log_table@sha256:777495cfb2f9f9ef79d57178c0e71bf69badc8e841b15a9b128c82d6a5e190f7, test:rust/crates/urzua-core/src/fix.rs#apply_repair_appends_a_plain_table_row_inside_the_plain_table_observed_failing@sha256:cfc41dee67c72b8cd5c8ee07cdd4abf1670853cc9cf89a8e250dd37e665e8deb
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
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 2 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#fix_apply_appends_inside_a_plain_revision_log_table` (lines 3368-3384) f67d384584d7 -> 777495cfb2f9, `rust/crates/urzua-core/src/fix.rs#apply_repair_appends_a_plain_table_row_inside_the_plain_table_observed_failing` (lines 1312-1328) c8c95bf3de4f -> cfc41dee67c7. Reason: Read the diff of each pinned span since the old pin and the claim still holds. check_integration.rs#fix_apply_appends_inside_a_plain_revision_log_table: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged; fix.rs#apply_repair_appends_a_plain_table_row_inside_the_plain_table_observed_failing: the call to apply_repair now passes the Attestation and revision-log shape arguments the function gained, the assertions on the written field and row are unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
