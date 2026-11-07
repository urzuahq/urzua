---
default: minor
closes: []
---

A record type can declare `statuses.delivery.items`, so that its Done means is a list whose items each
name their own evidence, and its `Status` is computed from the items instead of from one record-level
locator. An item ends in an evidence clause in the `Realized-by` grammar, or in an inline waiver with a
declared category, a cite and a reason. A record with every item resolved computes delivered, one with
some resolved computes partial, and `urzua fix --apply` writes the computed status (`InProgress` for
partial work, with the item counts in the revision row) and never downgrades a finished record.

A finished record with an unresolved item is a new error of `status.delivery-agreement`,
`DoneWithUnresolvedItems`. It names each unresolved item and its cause, fails `urzua check` under every
`fail_on` and every rule level, and cannot be cleared by a `WAIVER` record; only evidence or an item
waiver clears it. Two new rules, `status.done-items-shape` and `status.waived-item-stale`, hold the item
grammar and the waivers; item locators are checked by `embodiment.locator-exists`, read by drift and
re-pinned by `urzua fix --rebaseline`. `urzua roadmap` reports `items` counts for such a record, and
`urzua rules` prints a variant's `severity_floor` and `waivable`. A configuration that declares no
`items` is unchanged.
