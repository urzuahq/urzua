---
default: minor
closes: [MILE-197]
---

Adds a `verb_claims` option to `claim.status-agreement`. Set to `false`, it stops closing verbs in prose from being read as claims, so a change's `closes` field is its only claim; `check` states that the reading is off instead of counting verb claims. It is refused when `closes` is not declared, with either value, and on any other rule. Without `strict`, a notice says that a fragment lacking the key carries no claim. Left unset, the rule behaves as before.
