---
Stable-Id: 01M6D2KG0CMY0NH1R0P6HQA120
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 29"
Realized-by: test:rust/crates/urzua-cli/tests/check_integration.rs#fix_apply_preserves_crlf_and_the_absent_trailing_newline@sha256:281b018f5d80d967c2d5f57258b4851680ddde2f369016e374d902623a38e8cd, test:rust/crates/urzua-cli/tests/check_integration.rs#migrate_ids_apply_preserves_crlf_and_the_absent_trailing_newline@sha256:2d0a089cfa4c56519cc448c2fdbf0f1c048828ac32b60aaeae778470ebd56bce, test:rust/crates/urzua-core/src/fix.rs#apply_repair_on_a_crlf_file_changes_only_the_intended_lines_observed_failing@sha256:6bb3b938b90bbed0227903cb35f28216690962b242c7b82aba2c1efa3dcf59bc
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
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#fix_apply_preserves_crlf_and_the_absent_trailing_newline` (lines 3341-3366) d501ed6a1a5f -> 281b018f5d80, `rust/crates/urzua-cli/tests/check_integration.rs#migrate_ids_apply_preserves_crlf_and_the_absent_trailing_newline` (lines 3386-3403) f1dee1534f95 -> 2d0a089cfa4c, `rust/crates/urzua-core/src/fix.rs#apply_repair_on_a_crlf_file_changes_only_the_intended_lines_observed_failing` (lines 1330-1349) c50c1bfc8bd7 -> 6bb3b938b90b. Reason: Read the diff of each pinned span since the old pin and the claim still holds. check_integration.rs#fix_apply_preserves_crlf_and_the_absent_trailing_newline: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged; check_integration.rs#migrate_ids_apply_preserves_crlf_and_the_absent_trailing_newline: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged; fix.rs#apply_repair_on_a_crlf_file_changes_only_the_intended_lines_observed_failing: the call to apply_repair now passes the Attestation and revision-log shape arguments the function gained, the assertions on the written field and row are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
