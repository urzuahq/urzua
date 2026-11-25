---
Stable-Id: 01M88NWGSCAWGK1JZJEPJD0TWW
Status: Open
Found-in: the review of the pull request for BUG-232, reading the refusal of a sole locator against the pin-removed remedy
Realized-by:
---
# 234 — fix --pin --remove refuses a record's only locator, yet the pin-removed remedy names it

## What was wrong

`urzua fix --pin --remove OLD` refuses to remove a record's only locator: the evidence field's sole entry, or an item's sole entry, has no neighbouring comma, so `apply_remove` (`rust/crates/urzua-core/src/fix/pin.rs`) answers "not one of several evidence entries; remove a record's only locator by hand". `SPEC-8` states the same ("A record's only locator is removed by hand"). But a hand removal of a pin is a `pin-removed` finding of `diff.pin-changed-unattested`, and its remedy text names `--remove` for the case where the record no longer rests on the pin (`rust/crates/urzua-core/src/rules/pin_attestation.rs`, the `_` arm of the remedy near line 374; `SPEC-2`, the rule's description near line 377). So for a sole locator the finding names a command that refuses, the class of `BUG-230`, and the only way to clear the finding is a hand-written row that the tool exists to avoid.

It predates `BUG-232` and is not changed by it.

The same dead end has a second route: `fix --pin --remove` refuses a pin whose evidence field it cannot locate (a quoted key) and tells the user to remove the pin by hand, which is the hand removal that raises the `pin-removed` finding naming `--remove`.

## Why nothing caught it

The refusal and the remedy were written in different places, and no test follows the remedy of a `pin-removed` finding for a record whose removed pin was its only entry.

## References

Open. The fix is a decision: either `--remove` removes a sole entry, leaving the field empty or the item without its evidence clause (which can change a computed status, `ADR-96`), or the remedy names the hand removal and the row it needs when the pin is the only one. Found while fixing `BUG-232`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-25 | Added the second route into the dead end: the `--remove` refusal for a field it cannot locate tells the user to remove the pin by hand. | **structural** |
