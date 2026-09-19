---
default: patch
---

`claim.status-agreement` no longer raises false errors on ordinary prose, a
malformed rule setting says what is wrong with it, and `off` is declarable on a
rule that takes options.

`claim.status-agreement` matched a verb as a substring and claimed every
reference on the line (`BUG-45`), so `prefixes` read as `fixes` and
*"Fixes BUG-39, which RFC-9 predicted"* raised a blocking error about `RFC-9`.
Verbs now match on word boundaries, and a claim binds to the references the
verb governs.

A malformed rule setting said only that data did not match an untagged enum
(`BUG-46`). `RuleSetting` now dispatches on the parsed value, so the messages
naming the valid levels and keys are reachable again.

A quoted reference in prose (`'RFC-9'`) is recognised, and the unused `toml`
workspace dependency is removed.
