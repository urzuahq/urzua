---
default: minor
closes: []
---

Add `code.dangling-bug-reference`, an opt-in rule that reports a source comment citing a record id
that resolves to no record, the mirror of `embodiment.locator-exists`. It reads the tracked files
under its `back_pointer_paths` that have one of its declared `extensions`, takes the record prefixes
from the record types, judges existence only, leaves a back-pointer that
`embodiment.back-pointer-resolution` already judges to that rule, and takes `external_ids` and
`ignore_marker` as `citation.resolution` and the boundary rule do.
