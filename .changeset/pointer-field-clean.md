---
default: minor
---

# `check` now flags pointer fields that mix in explanatory prose

New rule, `header.pointer-field-clean`: `Implements`/`Derives-from`/`Parent` are meant to hold only
comma-separated reference IDs, but nothing previously caught a value like `RFC-1 (Accepted)` or
`SPEC-1 (v0 CLI), which lists the bug classes this suite must reproduce.` -- `extract_references`
silently reads the leading token and discards the rest. `Blocked-on` is deliberately excluded: it
legitimately mixes free text with an optional embedded reference.
