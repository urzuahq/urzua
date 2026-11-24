---
Stable-Id: 01M864JFZHYRNYNP036VF40Z0V
Status: Fixed
Found-in: a pull request that collapsed exact duplicate evidence lines in six milestones, where urzua check --base reported each collapsed copy as a removed pin
Realized-by: code:rust/crates/urzua-core/src/rules/pin_attestation.rs#unattested@sha256:c30c4aa14949b410d4fbe8f24df7b7d6c3b1a04bab60871047abd77ab6c89137, code:rust/crates/urzua-core/src/rules/pin_attestation.rs#collapsed@sha256:439bdec50c5f9b3186ff58ac948d9e04a14df744fbc38bbc97916591c9c4cd8b, test:rust/crates/urzua-core/src/rules/pin_attestation.rs#a_collapse_keeps_its_tier_and_its_pin_observed_failing@sha256:f12480d0e6de7fc82adebe3cc111b43be913e5e2c4ba17bcf54fb9846d46a1fd, test:rust/crates/urzua-core/src/rules/pin_attestation.rs#a_collapsed_duplicate_pin_is_not_a_removal_observed_failing@sha256:74420af48f5d2cb7d0038e7db70c583a7b3fc114e86a3ced92baaa6f810e698d
---
# 227 — a collapsed duplicate evidence pin is reported as a removed pin

## What was wrong

`diff.pin-changed-unattested` reported a record that collapsed an exact duplicate evidence line, the same locator at the same hash cited twice in the merge base and once in the head, as `pin-removed`: "the pinned locator ... was removed with no row". Nothing was un-pinned: the head still pins that locator at that hash, so the evidence is unchanged. The only way to clear the finding was `urzua fix --pin --remove` and a fresh pin, which writes a row that claims an un-pin that never happened, so a record carrying a duplicate could not be tidied honestly.

## Why nothing caught it

The rule compares base and head pins as multisets: each head pin pairs with one base pin, and a base pin left over is a removal. No test gave the base two copies of one pin and the head one, so the case where a leftover base pin is still pinned by the head was never exercised. The tests for removal all removed the last copy of a pin.

## References

Fixed in the engine, `rust/crates/urzua-core/src/rules/pin_attestation.rs` (`unattested`, `collapsed`): a base pin left unpaired needs no row when the head still pins its locator in the same tier (`code:`, `test:`) at the same hash, at any site, and leaves no more of that locator's citations unpinned than the base did. Which site lost its copy (the field or an item) is not recorded. A changed hash, a removal of the last copy, a collapse of two different hashes into one, a `test:` copy dropped while the `code:` copy of the same file stays, a copy kept with its pin taken off, a pin moved to another locator and a duplicate added in the head are each still reported. The planted tests are `a_collapsed_duplicate_pin_is_not_a_removal_observed_failing` and `a_collapse_keeps_its_tier_and_its_pin_observed_failing` in the same file; `SPEC-2` states the case under `diff.pin-changed-unattested`. The tier comparison also fixes `BUG-228`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 0 file-level, 2 scoped. Evidence: rust/crates/urzua-core/src/rules/pin_attestation.rs#unattested@sha256:99db48e7d559, rust/crates/urzua-core/src/rules/pin_attestation.rs#a_collapsed_duplicate_pin_is_not_a_removal_observed_failing@sha256:74420af48f5d Reason: unattested skips a leftover base pin whose locator and hash the head still pins; the planted test failed on the code before that check and passes after, and still fails a genuine removal, a changed hash and a collapse of two hashes **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-24 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 0 file-level, 1 scoped. Evidence: rust/crates/urzua-core/src/rules/pin_attestation.rs#a_collapse_keeps_its_tier_and_its_pin_observed_failing@sha256:f12480d0e6de Reason: unattested pairs pins and exempts a collapsed duplicate only within one tier, and not when a kept copy lost its pin; each planted test failed on the code before this change (the tier swap and tier-crossing collapse) or with the unpinned-count check removed (the stripped copy), and passes after **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-24 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 0 file-level, 1 scoped. Evidence: rust/crates/urzua-core/src/rules/pin_attestation.rs#collapsed@sha256:439bdec50c5f Reason: collapsed holds the duplicate exemption: same tier, locator and hash in the head, and no more unpinned copies than the base **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-24 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/pin_attestation.rs#unattested` (lines 277-350) 99db48e7d559 -> c30c4aa14949. Reason: Read the diff of unattested: it pairs a leftover head pin with a base pin only in the same tier and delegates the duplicate exemption to collapsed; the bug's claim, a collapsed duplicate is not a removal, still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
