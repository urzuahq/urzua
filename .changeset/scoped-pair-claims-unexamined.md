---
default: patch
closes: []
---

A relation claim to a record type outside a pair's `types` list no longer makes `config.scope-matches-nothing` warn that the configuration cannot reach it; the claim is counted as eligible and unexamined.
