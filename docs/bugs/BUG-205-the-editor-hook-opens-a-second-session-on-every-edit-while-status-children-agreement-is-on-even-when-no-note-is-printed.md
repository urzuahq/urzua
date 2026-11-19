---
Stable-Id: 01M7SKSRQZ82NZWMGKEV5GRX69
Status: Fixed
Found-in: "reading `harness/run.rs` after the editor hook gained the parent-state note, 0.4.12"
Realized-by: test:rust/crates/urzua-cli/tests/children_agreement_integration.rs#a_blocked_edit_still_names_the_parent_it_contradicts@sha256:5df92e4162e89c1ec46c754cf134bbd7dc8fae03da0ff792fe4c5dce684b669b
---
# 205 — The editor hook opens a second session on every edit while status.children-agreement is on, even when no note is printed

## What was wrong

With `status.children-agreement` on, the editor hook opened a second full session to compute the parent-state note before it judged the edit, so an edit the hook could not judge or that no rule governed paid for a note it never printed.

## Why nothing caught it

The note was computed first because it was simple to place, and its output is only visible on the paths that print it. No test can tell a note computed early from one computed late, so the regression test pins what must not change: the verdict, and the note on a blocked edit.

## References

The note is computed after the edit has been judged, and only while the rule is on. The saving is not observable from outside the process, so no test fails on the old code; `children_agreement_integration::a_blocked_edit_still_names_the_parent_it_contradicts` and `an_edit_that_moves_a_child_is_allowed_and_names_the_parent_it_contradicts` hold the verdict and the output unchanged.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
