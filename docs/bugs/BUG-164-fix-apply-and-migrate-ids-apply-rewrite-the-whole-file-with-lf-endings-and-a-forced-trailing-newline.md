---
Stable-Id: 01M6D2KG0CMY0NH1R0P6HQA120
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 29"
Realized-by: test:rust/crates/urzua-cli/tests/check_integration.rs#fix_apply_preserves_crlf_and_the_absent_trailing_newline@sha256:d501ed6a1a5f9cf20429f3c616709552356bbcf57fa83f8ead8b640068a40eb1, test:rust/crates/urzua-cli/tests/check_integration.rs#migrate_ids_apply_preserves_crlf_and_the_absent_trailing_newline@sha256:f1dee1534f958c4090c87f453fe8beceb7f81095c0c4562898ecfcbb063f371b, test:rust/crates/urzua-core/src/fix.rs#apply_repair_on_a_crlf_file_changes_only_the_intended_lines_observed_failing@sha256:c50c1bfc8bd78d56d95e2376508a29faf8eef9bab4bcfe2388f947cea6247477
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
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 3 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
