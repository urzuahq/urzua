---
default: patch
---

Fixes `BUG-111`: `parse_record_filename` supports a hyphenated type prefix (`DOC-ADR-2-x.md`), but the
reference recognizer used by `claim.status-agreement`, `pointer.resolution`/`narrative-field.stale`,
and `header.pointer-field-clean` split on only the first hyphen, so a citation of a hyphenated-prefix
record (`DOC-ADR-2`) was rejected as not-a-reference everywhere it was written. One shared predicate
now matches `parse_record_filename`'s own algorithm (last segment is the number, everything before it
is the prefix) at all three call sites.

Fixes `BUG-112`: the date-vs-record-number filename heuristic required zero-padded month/day segments,
so `2026-9-19-notes.md` (a genuine, just-unpadded date) parsed as record `2026` instead of being
recognized as a date.

Fixes `BUG-113`: `Population::detailed`'s `examined <= eligible` invariant was checked with
`debug_assert!`, which compiles to nothing in a release build — the exact profile `make ci` and an
adopter's own CI run. Now a real `assert!`.

Files `BUG-110` (`Status` is a hardcoded field-name literal rather than adopter-declared vocabulary,
needs a config-schema decision) without fixing it.

No adopter-facing behavior changes beyond the three fixes above: verified with the full test suite,
clippy, `make ci`, and a real-corpus `check` run reporting the same findings before and after. Four
other candidates from this review pass were investigated and refuted as deliberate, already-decided
design (untrimmed `Status` comparisons, a flat `terminal_statuses` config replacing a hardcoded
per-type table, `declared_slots`' pre-population type-declaration filter, and a hardcoded em-dash
placeholder check) or as premised on something that can't happen (an old `schema_version` config
loading silently rather than failing loudly).
