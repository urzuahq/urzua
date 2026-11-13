---
default: minor
closes: []
---

New opt-in rule `header.case-variant-key`: reports a record whose header writes two keys differing only
by case (`Status` and `status`), naming each spelling and its line. It needs no field declaration, so it
covers a record type that declares no vocabulary, and it reports the difference without saying which
spelling is right. Field names still compare exactly (`ADR-57`): an exact repeat of one spelling remains
`header.required-fields`' finding, and no other rule's verdict changes. Declare it under `rules:` at
`warn` or `error` to enable it; the run reports how many records it examined.
