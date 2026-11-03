---
Stable-Id: 01M6D2KG0CMY0NH1R0P6HQA120
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 29"
Regression-test: "fix_apply_preserves_crlf_and_the_absent_trailing_newline, migrate_ids_apply_preserves_crlf_and_the_absent_trailing_newline (rust/crates/urzua-cli/tests/check_integration.rs); apply_repair_on_a_crlf_file_changes_only_the_intended_lines_observed_failing (rust/crates/urzua-core/src/fix.rs)"
---
# 164 — fix --apply and migrate ids --apply rewrite the whole file with LF endings and a forced trailing newline

## What was wrong

`urzua-core::fix::apply_repair` and `urzua migrate ids --apply` both rebuilt the record as
`content.lines()` ... `lines.join("\n") + "\n"`. `lines()` discards each line's terminator, so a CRLF
record came back LF-only, and a file with no trailing newline gained one. A one-line change
(`Embodiment` recomputed, or one `Stable-Id` line inserted) became a whole-file diff: on a 17-line CRLF
fixture `migrate ids --apply` changed every line, and `fix --apply` showed 18 insertions and 17
deletions.

SPEC-8 states "The edit touches only the one field's line. Every other field and all prose is
untouched, byte-for-byte -- verified by test, not just intended." That held only for LF files with a
trailing newline, and the existing test used exactly that shape.

## Why nothing caught it

Every fixture in the suite was LF with a trailing newline, the one shape `lines().join("\n") + "\n"`
reproduces exactly. A repository that checks records out with CRLF (a Windows checkout, or
`core.autocrlf`) is the case the config-driven thesis has to serve, and none was exercised.

## Fix

A shared `urzua-core::line_edit::LineDoc` keeps each line together with its own terminator. Both
writers now replace or insert specific lines in it: replacing a line keeps its terminator, an inserted
line takes the file's first terminator, and a file without a trailing newline stays without one. Every
other byte is untouched. One mechanism for both commands, so the next writer starts from it rather
than from `lines().join`.

Verified failing before the fix (whole-file rewrite) and passing after, for both commands, on a CRLF
record with no trailing newline.

The revision-log row prefix (`BUG-165`) is the second symptom of the same rebuild-from-lines shape and
is fixed through the same helper.

## References

- `SPEC-8` -- "byte-for-byte" apply-mode promise this restores.
- `BUG-165` -- the hardcoded row prefix, same mechanism.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed and fixed in the same change. **Why:** found by a full-release code review and reproduced with the release binary on a disposable fixture; the regression test was observed failing before the fix and passing after. | **substantive** |
