---
default: minor
closes: [BUG-182]
---

`urzua new` now follows the record type's declared `identity` pattern, status vocabulary and field
set, so a generated record passes `check` with no edit. A type declaring a padded pattern such as
`\d{4}` gets a matching file name (`ADR-0004-slug.md`), an H1 that opens `ADR-4`, and numbering read
from that pattern. The status field starts at the first value of the type's declared `statuses`
instead of a literal `null`. `Stable-Id` is left out, with a warning notice and `"stable_id": null`
in the report, when the type's closed `known_fields` does not list it. A pattern no file name can
satisfy is refused before anything is written.
