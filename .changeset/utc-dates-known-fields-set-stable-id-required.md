---
default: patch
closes: []
---

Every date `urzua` compares is now a UTC date. A waiver's `Expires` and the date `urzua refs refresh`
writes into the snapshot were read from the machine's local clock, so a waiver could lapse a day
earlier or later depending on the contributor's time zone; claim ages already counted from UTC.

When `config.pointer-field-not-known` or `config.relation-field-not-known` names a record type that
declares no `known_fields`, the suggested `edit-config` now sets the full list of fields that type's
records use, plus the field in question, instead of appending one field. Appending created a one-item
list, which turned `header.field-set-consistency` on and flagged every other optional field.

`urzua new` keeps the generated `Stable-Id` for a record type that lists `Stable-Id` under
`required_fields`; it was overwritten with a blank.
