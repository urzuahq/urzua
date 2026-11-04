---
default: patch
---

`urzua init` no longer proposes `field.quality`, `field.pending`, `header.field-set-consistency` or
`header.pointer-field-clean`. Each reads a per-type declaration (`required_fields`, `known_fields` or
`pointer_fields`) that `init` does not write, so on a freshly adopted corpus they reported `ran` and
examined no record. The report now carries a `rule-applicability` notice naming the four rules and the
declaration each one needs, so the adopter can declare it and enable the rule.
