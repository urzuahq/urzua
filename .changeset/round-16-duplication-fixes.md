---
default: patch
---

`header_pointer_field_clean` and `narrative_field_stale` were missed by an earlier pass consolidating
duplicated `(record, field)` slot construction into the shared `field_slots` helper; both now use it,
matching every other `Field`-unit rule. A comment in `new_record.rs` using temporal language against
the project's comment convention is reworded to state the invariant directly. Both found by a
`/code-review v0.3.0...main` pass; no adopter-facing behavior change, verified with a real-corpus
`check` run reporting the same findings before and after.
