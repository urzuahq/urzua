---
Stable-Id: 01M7V83A16FGGTXWQFY9N3EKD7
Status: Fixed
Found-in: merging main into a pull request that narrowed whole-file pins to symbols and line ranges, where 19 of 24 conflicted records were refused by urzua fix --conflicts and resolved by hand
Realized-by: test:rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#a_locator_one_side_narrowed_and_the_other_re_pinned_resolves_to_the_narrowed_locator_observed_failing@sha256:b140269a51f7187bd137ba536f05439c60018575ac04df058214ea0ba12b48ac, test:rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#a_done_means_item_locator_one_side_narrowed_and_the_other_re_pinned_resolves_observed_failing@sha256:cedb1b948c28325b8f7408543d4afd166be9ab64b127c4cf293f09c93aed1f15, test:rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#a_locator_both_sides_replaced_differently_is_refused_naming_both@sha256:506f00dd567318922d052f0840aff7099a45b4b018298817222ad1e0ec27e905
---
# 210 — urzua fix --conflicts refuses a record where one side changed a locator and the other only re-pinned the old one

## What was wrong

`urzua fix --conflicts` refused a record whose two sides disagreed about a pinned locator: one side narrowed `code:path@sha256:X` to `code:path#symbol@sha256:Y` (or a line range) and the other only re-pinned the old `code:path@sha256:Z`. The refusal read "Done means item N differs between the two sides beyond its pin hashes", because the comparison of the two sides' Done-means items counted the pinned locator's spelling as a difference, so every milestone whose item cited a narrowed locator had to be resolved by hand. That is the case `MILE-284` meant the command to handle: a narrowing is exactly what makes two branches re-pin the same record. In the evidence field the narrowed locator was kept, but with the narrowing side's hash as written, which is stale when the other side changed the narrowed span.

## Why nothing caught it

No test changed a locator on one side. The tests for items masked every hash and then required the rest identical, so they showed a record with moved pins resolving and a record with an edited item refused, never a pinned locator replaced by another. The refusal was also written into `SPEC-8` as the rule, so it read as intended behaviour rather than a gap.

## References

Fixed in `rust/crates/urzua-core/src/conflicts.rs` (`pair_moved_locators`, `MovedFrom`; the items comparison masks a pinned locator's spelling) and `rust/crates/urzua-cli/src/commands/merge_pins.rs` (`attested_sides`): the locator one side changed wins, in the evidence field and in an item alike, and its pin is recomputed from the merged file under its scope, each side attested by its own locator. A locator both sides replaced with different ones is refused naming both. `SPEC-8` ("A locator one side changed") and the `ADR-67` amendment of 2026-11-20 record the decision, and the case `a-locator-one-side-narrowed-and-the-other-re-pinned-resolves-on-merge` in `corpus-aj` reproduces it over a real merge.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-20 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 0 file-level, 3 scoped. Evidence: rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#a_locator_one_side_narrowed_and_the_other_re_pinned_resolves_to_the_narrowed_locator_observed_failing@sha256:b140269a51f7, rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#a_done_means_item_locator_one_side_narrowed_and_the_other_re_pinned_resolves_observed_failing@sha256:cedb1b948c28, rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#a_locator_both_sides_replaced_differently_is_refused_naming_both@sha256:506f00dd5673 Reason: the three tests are the planted violations for the locator change: each failed before the fix and passes after **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
