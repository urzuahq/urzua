---
default: patch
closes: []
---

`config.relation-field-not-known` also reports a `relation_fields.date` or `relation_fields.author`
that is not in the type's `required_fields` or `known_fields`, as it already did for the relation roles.
Which rules accept which options is read from the rule table; the load errors are unchanged.
