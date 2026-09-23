---
default: patch
---

`urzua migrate schema --report` reported every record of a `header_shape: none` type as `Unreadable`
with a spurious "header did not parse" notice, excluding it from the preview -- such a type has no
header to parse by construction, and can never legally declare the candidate field at all (`ADR-50`).
`schema_report` now gates on the same `has_no_header()` check every other header-parse call site
already uses, and excludes such records from the preview without treating them as a defect.
