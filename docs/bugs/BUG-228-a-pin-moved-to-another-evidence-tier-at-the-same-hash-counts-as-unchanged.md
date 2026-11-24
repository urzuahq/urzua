---
Stable-Id: 01M865CZKBWAJR47YN9SPSMEMY
Status: Fixed
Found-in: an independent review of the pull request fixing BUG-227, which noted that diff.pin-changed-unattested compares locators without their tier
Realized-by: code:rust/crates/urzua-core/src/rules/pin_attestation.rs#unpaired@sha256:5abe59ab49cd223a424bb26da734a8b94645b21bfece068d701cc87e37161bab, test:rust/crates/urzua-core/src/rules/pin_attestation.rs#a_pin_moved_to_another_tier_is_not_unchanged_observed_failing@sha256:f6b2d55be50939ad2037af97bab8168739658d99afcc50d7d7e8f10df4154bd4
---
# 228 — a pin moved to another evidence tier at the same hash counts as unchanged

## What was wrong

`diff.pin-changed-unattested` read each evidence citation's locator through `Locator::display()`, which keeps the path and scope and drops the tier. A pin moved from `code:src/a.rs@sha256:X` to `test:src/a.rs@sha256:X` was paired as unchanged and passed `urzua check --base` with no row, though it changes what kind of evidence the record claims. Records that cite one file under two tiers are common in this corpus (`ADR-17`, `ADR-19`, `ADR-20`, `ADR-21`, `ADR-22`, `ADR-24` and `ADR-27` do), so the swap is a real edit, not a theoretical one.

## Why nothing caught it

Every test of the rule built its citations without a tier, so two citations of one file under different tiers never met. The pairing reads the locator the reports print, and the printed form had never needed the tier.

## References

Fixed in `rust/crates/urzua-core/src/rules/pin_attestation.rs`: `Pin` carries the entry's category, and `unpaired`, the change pairing in `unattested` and `collapsed` compare it, so a tier swap reports `pin-added-unattested` and `pin-removed`, cleared by `urzua fix --pin --apply --replace OLD=NEW`. The rows the pinning tools write still name a locator without its tier, so a row accounts for a locator in any tier; that is unchanged and filed as `BUG-229`. The planted test is `a_pin_moved_to_another_tier_is_not_unchanged_observed_failing`; `SPEC-2` states that pins pair only within one tier.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 0 file-level, 2 scoped. Evidence: rust/crates/urzua-core/src/rules/pin_attestation.rs#unpaired@sha256:5abe59ab49cd, rust/crates/urzua-core/src/rules/pin_attestation.rs#a_pin_moved_to_another_tier_is_not_unchanged_observed_failing@sha256:f6b2d55be509 Reason: unattested pairs pins and exempts a collapsed duplicate only within one tier, and not when a kept copy lost its pin; each planted test failed on the code before this change (the tier swap and tier-crossing collapse) or with the unpinned-count check removed (the stripped copy), and passes after **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
