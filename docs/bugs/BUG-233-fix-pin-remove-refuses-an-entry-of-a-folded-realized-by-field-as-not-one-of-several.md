---
Stable-Id: 01M88KTR7YQ947KFYPD7Y6MZA4
Status: Fixed
Found-in: the review of the pull request for BUG-232, which asked how --remove handles a folded Realized-by field
Realized-by: code:rust/crates/urzua-core/src/fix/pin.rs#remove_entry@sha256:ba186af3b5ec315a356f842b4635ab199c45639cff0f7f6eceb0d511e0320a63, test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#fix_pin_remove_reads_a_folded_field_observed_failing@sha256:98a8dae741ee93728d23a49a15577a40c94a098d6265a35e9c73893f111b441c
---
# 233 — fix --pin --remove refuses an entry of a folded Realized-by field as not one of several

## What was wrong

`urzua fix --pin --remove OLD` found the entry to remove only as `, <pin>` or `<pin>, ` on one line (`apply_remove` in `rust/crates/urzua-core/src/fix/pin.rs`). A `Realized-by: >-` field that puts each entry on its own line separates them with `,` and a newline, so neither form matched and the command refused with "not one of several evidence entries; remove a record's only locator by hand" for an entry that had a neighbour.

## Why nothing caught it

No test of `fix --pin --remove` wrote the field as a folded scalar.

## References

Fixed in `remove_entry`, which finds the comma on either side across a line break and drops the entry's own line. The tests are `fix_pin_remove_reads_a_folded_field_observed_failing` in `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` and the acceptance case `fix-pin-removes-an-entry-of-a-folded-field` in `corpus-g`, each observed failing before the fix.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-25 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 0 file-level, 2 scoped. Evidence: rust/crates/urzua-core/src/fix/pin.rs#remove_entry@sha256:e199a7f1dbf7, rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#fix_pin_remove_reads_a_folded_field_observed_failing@sha256:f5b0238dbc1d Reason: BUG-233 pins remove_entry, which reads a comma across a line break, and the folded-field test that failed before it. **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-25 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix/pin.rs#remove_entry` (lines 172-200) e199a7f1dbf7 -> ba186af3b5ec, `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#fix_pin_remove_reads_a_folded_field_observed_failing` (lines 2087-2101) f5b0238dbc1d -> 98a8dae741ee. Reason: Read the diff: remove_entry skips a match on a comment line and strips a CRLF line break, and the folded-field test gains a check --base assertion; the folded-field behaviour cited is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
