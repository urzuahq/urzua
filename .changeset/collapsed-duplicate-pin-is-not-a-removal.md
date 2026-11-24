---
default: patch
closes: [BUG-227]
---

`urzua check --base` no longer reports `pin-removed` when a record collapses an exact duplicate evidence line, the same locator at the same hash cited twice, into one. The head still pins that locator at that hash, so no row is needed. A changed hash, the removal of a pin's last copy and a duplicate added in the head are still reported.
