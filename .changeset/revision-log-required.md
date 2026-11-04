---
default: minor
---

A record type's `revision_log` declaration takes `required: true|false` (`true` when omitted). With
`required: false`, `urzua fix --apply` writes a computed field into a record that has no revision log
section, without adding a log row; the record type needs no `marker` or `heading` for that, so
`revision_log: {required: false}` is enough. Where the log is required, a record without the section
still fails the write, and the error now names the marker or heading it looked for.
