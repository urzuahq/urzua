---
default: patch
closes: []
---

A milestone whose evidence is its Done-means item clauses, with no `Realized-by` line, now expires under `embodiment.claim-expired` and is renewed by `urzua reverify` like any other claim (`ADR-97`). Its age counts from the first evidence line, `reverify` pins the item locators, and a record with neither a `Realized-by` value nor an item evidence clause stays not examined with the reason named. A record that states `Realized-by` is judged as before.
