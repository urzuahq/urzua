---
Stable-Id: 01M866AFEF8KFCCAAGE600V9BE
Status: Fixed
Found-in: an independent review of the pull request fixing BUG-227 and BUG-228, on a record citing one file as code and as test at the same hash
Realized-by: code:rust/crates/urzua-cli/src/commands/fix_pin.rs#pinned_entry@sha256:8d73f53f7f87dbf1eda210d5e20b2076ef443b023f34d5cf32bbd5c21afe65b8, test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#fix_pin_removes_one_tier_of_a_file_cited_in_two_observed_failing@sha256:1576928c5edc24462be17fc5fbd0b1002225e226db0fae2c1fe73521c4a0732f
---
# 230 — fix pin remove and replace refuse a file cited in two tiers at one hash

## What was wrong

`urzua fix --pin --apply --remove OLD` and `--replace OLD=NEW` find `OLD` among the record's pinned citations by its path and scope only (`pinned_entry` in `rust/crates/urzua-cli/src/commands/fix_pin.rs`), then require every match to be the same entry, tier included. On a record citing `code:X@sha256:H` and `test:X@sha256:H` the two matches differ only in tier, so the command refused with "'X' is pinned at more than one hash", though both hashes are equal. Since `BUG-228`, dropping one of those citations by hand is a `pin-removed` finding whose remedy is this command, so the finding named a command that could not run.

## Why nothing caught it

No test of `fix --pin` cited one file in two tiers, and `OLD` had no way to name a tier: the comparison was written when the attestation rule ignored tiers too.

## References

Fixed in `pinned_entry`: `OLD` may carry its tier (`code:X`, `test:X`, or any evidence category), which narrows the match to that tier, and a bare `OLD` that matches citations in more than one tier is refused with a message naming both spellings rather than a false "more than one hash". The tests are `fix_pin_removes_one_tier_of_a_file_cited_in_two_observed_failing` in `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` and the acceptance case `fix-pin-removes-one-tier-of-a-file-cited-in-two` in `corpus-g`, each observed failing before the fix. `SPEC-8` ("Pinning evidence with a row") states the tier prefix. The row the command writes still names no tier (`BUG-229`).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 0 file-level, 2 scoped. Evidence: rust/crates/urzua-cli/src/commands/fix_pin.rs#pinned_entry@sha256:96493a826674, rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#fix_pin_removes_one_tier_of_a_file_cited_in_two_observed_failing@sha256:1576928c5edc Reason: pinned_entry narrows OLD to the tier it names and refuses a bare OLD matching two tiers with both spellings; the integration test and the corpus-g case failed before this change (partial-failure, 'pinned at more than one hash') and pass after **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-24 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix_pin.rs#pinned_entry` (lines 389-431) 96493a826674 -> 8d73f53f7f87. Reason: Read the diff of pinned_entry: only the more-than-one-tier refusal changed, now built from the tiers found; the tier narrowing BUG-230 describes is untouched. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
