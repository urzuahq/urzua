---
default: minor
closes: []
---

The `embodiment.locator-promotion-candidate` rule accepts `scope: changed`.

With the option, a locator cited by more than one record is reported only when a record the
`--base` diff adds or changes cites it. The finding sits on that changed record and names every
citer, changed or not. A plain `check` reports none, and the records it did not judge are counted
in the population's `deferred_by_scope`.
