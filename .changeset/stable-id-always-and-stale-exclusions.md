---
default: patch
---

`urzua new` always writes a `Stable-Id` and reports it in `stable_id`; when the record type's closed
`known_fields` does not list `Stable-Id`, it adds a warning notice naming that configuration
inconsistency. `type.dir-matches-nothing` now also warns about an `exclude_files` entry that matches
no file directly in the type's `dir`, so a stale exclusion is visible.
