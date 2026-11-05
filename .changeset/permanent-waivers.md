---
default: minor
closes: []
---

A suppression type can declare `permanent`, which says per rule which reasons let a waiver without
`Expires` take effect. Each entry names a rule, a `reason_field` the type declares, a non-empty list of
`categories` (each optionally `cite: required`) and, for a category that requires one, a `cite_field`
that is a pointer field. A waiver with no `Expires` then suppresses only for a listed rule, with a
category from that rule's list and, where required, a citation that resolves to a record. Declaring the
key also closes permanence for every rule it does not list. Otherwise the waiver suppresses nothing, its
finding is reported again, and `waiver.matches-nothing` reports it as `waiver-ineffective` with a message
that now names whichever requirement is unmet instead of always naming `effective_requires`. `check`
adds an info notice, `permanent-waivers`, counting the permanent waivers in effect by category. A waiver
with an `Expires`, and a config that declares no `permanent`, behave as before. Config load refuses an
unknown rule, a field the type does not declare, an empty or duplicated category list and a `cite_field`
that is not a pointer field.
