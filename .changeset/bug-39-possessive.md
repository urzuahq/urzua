---
default: patch
---

A reference with a possessive attached is no longer invisible (`BUG-39`).

`MILE-13` in this repository declares `Blocked-on: RFC-9's own Q2`. Both
extractors returned nothing -- `extract_references` rejected `9's` as
non-digits, and `scan_references` trims only non-alphanumerics from each end,
so the `s` kept the apostrophe attached. A real dependency was invisible, and
`narrative-field.stale` reported success on a field it could not read.

Narrowed deliberately: only a possessive is stripped. `RFC-9a` and `RFC-9s` are
different identifiers and stay unrecognised.
