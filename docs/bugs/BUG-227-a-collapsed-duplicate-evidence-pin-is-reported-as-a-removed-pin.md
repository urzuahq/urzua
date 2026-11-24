---
Stable-Id: 01M864JFZHYRNYNP036VF40Z0V
Status: Fixed
Found-in: a pull request that collapsed exact duplicate evidence lines in six milestones, where urzua check --base reported each collapsed copy as a removed pin
Realized-by: code:rust/crates/urzua-core/src/rules/pin_attestation.rs#unattested@sha256:99db48e7d559e8d9f766f2670eab0917646cddbde274cad76538c6d41c432ce5, test:rust/crates/urzua-core/src/rules/pin_attestation.rs#a_collapsed_duplicate_pin_is_not_a_removal_observed_failing@sha256:74420af48f5d2cb7d0038e7db70c583a7b3fc114e86a3ced92baaa6f810e698d
---
# 227 — a collapsed duplicate evidence pin is reported as a removed pin

## What was wrong

`diff.pin-changed-unattested` reported a record that collapsed an exact duplicate evidence line, the same locator at the same hash cited twice in the merge base and once in the head, as `pin-removed`: "the pinned locator ... was removed with no row". Nothing was un-pinned: the head still pins that locator at that hash, so the evidence is unchanged. The only way to clear the finding was `urzua fix --pin --remove` and a fresh pin, which writes a row that claims an un-pin that never happened, so a record carrying a duplicate could not be tidied honestly.

## Why nothing caught it

The rule compares base and head pins as multisets: each head pin pairs with one base pin, and a base pin left over is a removal. No test gave the base two copies of one pin and the head one, so the case where a leftover base pin is still pinned by the head was never exercised. The tests for removal all removed the last copy of a pin.

## References

Fixed in the engine, `rust/crates/urzua-core/src/rules/pin_attestation.rs` (`unattested`): a base pin left unpaired whose exact locator and hash the head still pins needs no row. A changed hash, a removal of the last copy, a collapse of two different hashes into one, a pin moved to another locator and a duplicate added in the head are each still reported. The planted test is `a_collapsed_duplicate_pin_is_not_a_removal_observed_failing` in the same file; `SPEC-2` states the case under `diff.pin-changed-unattested`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 0 file-level, 2 scoped. Evidence: rust/crates/urzua-core/src/rules/pin_attestation.rs#unattested@sha256:99db48e7d559, rust/crates/urzua-core/src/rules/pin_attestation.rs#a_collapsed_duplicate_pin_is_not_a_removal_observed_failing@sha256:74420af48f5d Reason: unattested skips a leftover base pin whose locator and hash the head still pins; the planted test failed on the code before that check and passes after, and still fails a genuine removal, a changed hash and a collapse of two hashes **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
