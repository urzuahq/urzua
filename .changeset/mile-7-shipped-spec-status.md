---
default: patch
closes: []
---

This repository's `spec` record type now declares a `Shipped` status after `Accepted`, and requires `Embodiment` and
`Realized-by` of a spec in that status. It is configuration only: the same `statuses.require` declaration any type can
use, with no change to the CLI's behaviour for other configurations.
