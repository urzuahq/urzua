---
default: patch
closes: []
---

`embodiment.locator-exists` reports `Realized-by` text that is not a `spec:`, `code:` or `test:` entry
instead of ignoring it. `header.field-set-consistency` suggests renaming a key that differs from a
declared one only in case, never removing it. `date.field-valid` rejects a `date_fields` entry no
record type declares. `revision-log.change-class-required` reads markdown alignment rows
(`|:---|:---:|`) as delimiters, and `header.required-fields` no longer judges types that declare
no required fields.
