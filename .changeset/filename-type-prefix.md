---
default: minor
---

# `urzua new` now emits type-prefixed, unpadded filenames

New records are named `TYPE-N-slug.md` (e.g. `ADR-36-...md`) instead of `NNNN-slug.md` --
matching how a record is already referenced everywhere else (`Implements: ADR-36` is now the same
string as the file's own name). The number is never zero-padded: fixed-width padding doesn't solve
lexicographic sort order permanently, it only defers the break to whenever one type crosses the
padded width, and breaks worse there (mixed-width filenames). Unpadded is at least consistent
forever. The descriptive slug is kept.

Existing `NNNN-slug.md` filenames are never renamed and keep working forever -- both shapes resolve
identically regardless of padding, and a directory can hold a mix of both indefinitely. Also fixed:
`filename.title-consistency` had its own separate hardcoded 4-digit-only parsing (the same defect
class as BUG-2, in a different function) -- both now accept any digit length and compare by numeric
value.
