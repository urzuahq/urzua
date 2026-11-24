---
Stable-Id: 01M86780WXMN7RFMD5GQZZMP4J
Status: Fixed
Found-in: the review of the pull request that fixed BUG-230, reading `apply_remove` for a pin cited in the `Realized-by` field and again in a Done-means item
Realized-by: code:rust/crates/urzua-core/src/fix/pin.rs#apply_remove@sha256:9bbc23b6b352021b404b046a1434d2583fc43b5d408fdc855f0965088d076867, test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#fix_pin_remove_edits_the_field_copy_not_the_done_means_item_copy_observed_failing@sha256:5adfa2767a7a77e1667c704c75854fd2dde56e1cb0ff26f0aba33f53eaa1cde4
---
# 232 — fix --pin --remove edits a Done-means item's copy of a pin instead of the Realized-by field's

## What was wrong

`apply_remove` in `rust/crates/urzua-core/src/fix/pin.rs` searched the whole body before the revision log for `, <pin>` and then `<pin>, `, and removed every occurrence of the first form it found. On a record that cites the same pin in the `Realized-by` field and in a Done-means item, where the item's copy has the `, <pin>` form and the field's has the `<pin>, ` form, `--remove` edited the item and left the field's copy: the command reported the pin removed from the evidence the caller named while the item lost a locator it never named. It matches on tier, path and hash, so it never touched a different pin. `--replace` was checked in the same case and is not affected: it rewrites every copy, so field and item move together.

Decision (from `SPEC-8` and `ADR-96`): `--remove` removes the first copy of the pin in the record, which is the `Realized-by` field's, because the field precedes the body. A copy in a Done-means item stays. `ADR-96` makes each item its own criterion with its own evidence clause, and the engine never rewrites an item to fit a change made elsewhere; dropping a locator from an item is a change of that criterion, made by a person. When the pin is only in an item, `--remove` removes it from that item, the one copy there is.

## Why nothing caught it

No test of `fix --pin --remove` had a record with Done-means items, so the search over the whole evidence text was never run with two copies of one pin in different forms.

## References

Fixed in `apply_remove`: it removes the first copy only. The tests are `fix_pin_remove_edits_the_field_copy_not_the_done_means_item_copy_observed_failing`, observed failing before the fix (the item lost its locator and the field kept it), and `fix_pin_replace_moves_the_field_copy_and_the_item_copy`, which passed before and after and fixes the `--replace` behaviour. The acceptance case `fix-pin-remove-leaves-a-done-means-items-copy-of-the-pin` in `corpus-g` puts the pin alone in the field and again in an item: the old code edited the item and applied, the fixed code refuses to empty the field, and the case was observed failing before the fix.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 0 file-level, 2 scoped. Evidence: rust/crates/urzua-core/src/fix/pin.rs#apply_remove@sha256:9bbc23b6b352, rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#fix_pin_remove_edits_the_field_copy_not_the_done_means_item_copy_observed_failing@sha256:29f265a592d6 Reason: BUG-231 pins pinned_entry and its refusal test; BUG-232 pins apply_remove and the field-copy test, the code and tests this PR changed. **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-24 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#fix_pin_remove_edits_the_field_copy_not_the_done_means_item_copy_observed_failing` (lines 2037-2057) 29f265a592d6 -> 5adfa2767a7a. Reason: Read the diff of the test file since the first pin: cargo fmt reflowed the new tests' argument lists and one assert; their logic is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
