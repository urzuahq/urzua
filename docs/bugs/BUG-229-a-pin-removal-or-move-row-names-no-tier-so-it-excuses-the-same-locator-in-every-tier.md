---
Stable-Id: 01M865PQQVDP77MEHZ45J0TVKS
Status: Open
Found-in: an independent review of the pull request fixing BUG-227 and BUG-228, after pins were made to pair only within one tier
Realized-by:
---
# 229 — a pin removal or move row names no tier so it excuses the same locator in every tier

## What was wrong

The rows `urzua fix --pin --apply --remove` and `--replace` write name the locator as `locator_name` prints it (`rust/crates/urzua-core/src/fix/pin.rs`, `locator_name`, through `Locator::display()`), which drops the tier. `diff.pin-changed-unattested` reads those rows back and matches them on locator and hash alone (`rust/crates/urzua-core/src/rules/pin_attestation.rs`, `removed_ok` and `added_ok`). A record that cites `code:src/a.rs@sha256:X` and `test:src/a.rs@sha256:X` can therefore have both removed under one `--remove` row written for the `code:` entry, and the `test:` removal passes `urzua check --base` with no row of its own. The same holds for the old side of a `--replace`.

## Why nothing caught it

The rule compared no tier at all until `BUG-228`, so a row with no tier matched everything the rule could tell apart. Once pins paired within one tier, the rows became the one place the tier is still lost, and no test removes one tier's copy of a file under a row written for the other.

## References

Open. The fix needs the pinning tools to write the tier into their rows and the rule to match it, while still reading the rows already committed without one; that is a change to the row format, not only to the rule. Found while fixing `BUG-228`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
