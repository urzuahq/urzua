---
default: minor
---

# `urzua new` now emits type-prefixed filenames

New records are named `TYPE-NNNN-slug.md` (e.g. `ADR-0036-...md`) instead of `NNNN-slug.md` --
matching how a record is already referenced everywhere else (`Implements: ADR-0036` is now the same
string as the file's own name). Zero-padding and the descriptive slug are both kept.

Existing `NNNN-slug.md` filenames are never renamed and keep working forever -- both shapes resolve
identically, and a directory can hold a mix of both indefinitely. Also fixed: `filename.title-consistency`
had its own separate hardcoded 4-digit-only parsing (the same defect class as BUG-0002, in a
different function) -- both now accept any digit length and compare by numeric value.
