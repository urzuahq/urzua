---
default: minor
---

A record type can declare how its revision log is found with `revision_log`: a `marker` line (default
`**Revision log**`) or a `heading`. The log is the section that line opens, never a line that merely
mentions the text, so a record whose prose contains the literal is no longer read against the wrong
table. `revision-log.change-class-required`, `urzua fix` and `citation.path-exists` all read the
declared identity. Existing repositories are unchanged.
