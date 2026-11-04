---
default: minor
---

Set severity per field within one rule. `field.quality`, `field.pending` and
`status.required-field-missing` take an optional `field_levels` map from a field name to `warn` or
`error`, so a missing `Deciders` on an accepted record can block while a missing `Reviewers` only
warns. A field the map does not name keeps the rule's `level`, and a config without the option
behaves as before.
