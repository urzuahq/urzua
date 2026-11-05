---
default: patch
closes: []
---

A panic now exits with code 2, the documented code for "could not run", instead of 101, and a closed
stdout pipe (`urzua check | head -1`) ends the run quietly instead of panicking. `urzua new` fills a
record's date and author from the field names a type declares under `relation_fields.date` and
`relation_fields.author` (defaulting to `Date` and `Author`), so a type that calls them `Created` and
`Owner` gets them filled instead of `null`.
