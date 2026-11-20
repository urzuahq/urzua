---
Stable-Id: 01M7VK14SZXM9VK7QA6RTJ0KER
Status: Fixed
Found-in: a third review of the pull request that fixed BUG-210, which showed the ordinary path (both sides re-pinned one range) had the same hole as the new locator-change path
Realized-by: test:rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#two_range_re_pins_where_a_side_edited_outside_its_range_are_refused_observed_failing@sha256:a7b71b0fe605c5425ac84453d69e5377dc1e65020cddc3ac9e483bd224da84cb, test:rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#two_range_re_pins_the_merge_shifts_onto_an_unattested_edit_are_refused_observed_failing@sha256:51673935daead5f2302cbde09ee2f32c21cdea6f1876c594a746f367d102358a
---
# 211 — urzua fix --conflicts pins a range over lines nobody attested when a merge shifts the lines or a side edited outside its range

## What was wrong

`urzua fix --conflicts` recomputed a pin both sides had moved over the merged file without asking whether the merged span was what either side pinned. Two ways it pinned lines no side attested: a side had edited the file outside its pinned range or symbol (the merged file carries that edit, and a merge that shifts the range can bring it inside), and the other side's edits above a range moved it onto different lines. Example: one branch edits lines 5 and 9 and re-pins `src/a.rs#L5-L7`; the other deletes lines 1-3 and re-pins `#L5-L7`. The record resolved, and `#L5-L7` was pinned over the old lines 8-10, which hold the first branch's unpinned edit. The same hole made a narrowing resolved by the locator-change rule (`BUG-210`) unsound.

## Why nothing caught it

The attestation checks proved each side's file hashed to its pin, and that the merged file was git's clean merge of the two. Neither says the span the merged pin covers is the span either side read: a side's pin covers its span only, and a range names lines by number. The tests re-pinned whole files, where every line is covered, so no scoped pin was ever merged over another side's edit.

## References

Fixed in `rust/crates/urzua-cli/src/commands/merge_pins.rs` (`check_confined`): for every side whose attestation is scoped, that side's file must equal the merge base outside the scope's span, and for a range the merged file must keep the side's lines before it; otherwise the record is refused naming the side. `SPEC-8` ("A scoped pin is confined") and the `ADR-67` amendment of 2026-11-20 state it, and the case `two-range-re-pins-the-merge-shifts-onto-an-unattested-edit-are-refused` in `corpus-ak` reproduces it over a real merge.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-20 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 0 file-level, 2 scoped. Evidence: rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#two_range_re_pins_where_a_side_edited_outside_its_range_are_refused_observed_failing@sha256:a7b71b0fe605, rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#two_range_re_pins_the_merge_shifts_onto_an_unattested_edit_are_refused_observed_failing@sha256:51673935daea Reason: planted tests for the confinement of a recomputed scoped pin: each failed on the code before check_confined, and the (a) and (b) mutations turn them red **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
