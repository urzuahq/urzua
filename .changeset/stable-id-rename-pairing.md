---
default: patch
closes: []
---

Fix `diff.decider-required` and `status.transition-allowed` pairing a new record with a deleted one that merely
shares its `Stable-Id`. A record the base lacks is now judged against a deleted record only when the record id
(type and number) also matches, so a slug rename still keeps its status while a new Accepted record that copies a
deleted record's `Stable-Id` is a gated move that needs a decider.
