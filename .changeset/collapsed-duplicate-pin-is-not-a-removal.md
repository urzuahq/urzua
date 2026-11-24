---
default: patch
closes: [BUG-227, BUG-228]
---

`urzua check --base` no longer reports `pin-removed` when a record collapses an exact duplicate evidence citation, the same tier, locator and hash cited twice, into one. The head still pins it, so no row is needed. A changed hash, the removal of a pin's last copy, a copy of another tier, a copy kept with its pin taken off and a duplicate added in the head are still reported.

`diff.pin-changed-unattested` compares a citation's tier: a pin moved from `code:` to `test:` at the same hash was treated as unchanged and is now reported as added and removed, cleared with `urzua fix --pin --apply --replace OLD=NEW`.
