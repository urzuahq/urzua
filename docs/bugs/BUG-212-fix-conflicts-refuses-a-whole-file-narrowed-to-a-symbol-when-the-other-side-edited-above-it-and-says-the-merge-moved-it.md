---
Stable-Id: 01M7VSWDBECW6WQBGE7W9P1NQS
Status: Fixed
Found-in: the fifth review of the pull request that fixed BUG-210 and BUG-211, which showed the prefix check refused the most common narrowing merge for an edit above the symbol
Realized-by: test:rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#a_whole_file_narrowed_to_a_symbol_resolves_when_the_other_side_edited_above_it_observed_failing@sha256:0267d436733215ceec00d4eb51f40de6e786abb65a92c1d0e7515695e2a56a33, test:rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#a_range_with_edited_lines_above_it_is_refused_saying_the_other_side_edited_above_observed_failing@sha256:417f673bb981e2e03953f78bf46e24793b7bf6d1f6e48a4787abbbeb1fb2de0b, test:rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#one_dropped_locator_two_new_ones_replace_is_refused_observed_failing@sha256:12233a3bb169bfdbb72a87ff5955a05d23ea8695602c9fed14e6763db7f95f1f
---
# 212 — urzua fix --conflicts refuses a whole file narrowed to a symbol when the other side edited above it, and says the merge moved it

## What was wrong

`urzua fix --conflicts` refused a record where one side narrowed a whole-file pin to a symbol (`code:src/a.rs` to `code:src/a.rs#g`) and the other side edited the same file above that symbol and re-pinned the whole file. The recompute held the lines above the merged span equal to the narrowing side's, so any edit above the symbol, even one that moved no line, stopped the merge. The refusal said "the merge moved it", which was false: the other side had edited above it. A symbol resolves by name, the whole-file re-pin attests every line and the changing side is confined to its scope, so nothing about the lines above a symbol needed holding. For a range, which names lines by number, the check is needed and stays.

## Why nothing caught it

`BUG-211` added the check for ranges and symbols alike, and its tests for the narrowing case used ranges, so no test narrowed to a symbol while the other side edited above it. When a test for two narrowings failed on the check, its second side was changed to avoid the edit above, not the check questioned.

## References

Fixed in `rust/crates/urzua-cli/src/commands/merge_pins.rs` (`check_merged_span`): with a whole-file attestation on the other side, only a range must keep the lines above it; when both sides re-pinned a scope, the lines above and below must equal each side's. The refusals name what happened (the other side edited or moved lines above a range; the merged span does not end where the side's does). `SPEC-8` ("A scoped pin is confined") states it, and the case `a-whole-file-narrowed-to-a-symbol-resolves-when-the-other-side-edited-above-it` in `corpus-al` reproduces it over a real merge. A test for the branch of `ambiguous_pairing` that refuses one dropped locator shared by two new ones is pinned here too.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-20 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 0 file-level, 3 scoped. Evidence: rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#a_whole_file_narrowed_to_a_symbol_resolves_when_the_other_side_edited_above_it_observed_failing@sha256:24ab31c16e47, rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#a_range_with_edited_lines_above_it_is_refused_saying_the_other_side_edited_above_observed_failing@sha256:417f673bb981, rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#one_dropped_locator_two_new_ones_replace_is_refused_observed_failing@sha256:12233a3bb169 Reason: three planted tests: a whole file narrowed to a symbol resolves with an edit above it, a range with an edit above it is refused with the right wording, and one dropped locator shared by two new ones is refused; red on the previous code or under mutation **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-20 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#a_whole_file_narrowed_to_a_symbol_resolves_when_the_other_side_edited_above_it_observed_failing` (lines 2125-2138) 24ab31c16e47 -> 0267d4367332. Reason: the test's fixture helper lost its always-zero edit-line parameter, so the test calls it with one argument; the setup and the assertions are unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
