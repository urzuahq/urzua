---
Stable-Id: 01M6JW44TCEWCNANCN4X8ZZ8K0
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/status_model.rs, test:rust/crates/urzua-cli/tests/placeholder_vocabulary_integration.rs
---
# 162 — `statuses.pending` supports declared pattern matching for placeholder families

## What

Let `statuses.pending` (`MILE-153`) list patterns as well as whole values, so a family of placeholder statuses such as a prefix followed by a variable tail is declared once.

## Why

A real adopter corpus uses placeholder values that share a stem and differ in a tail, so an exact-value list needs an entry per variant and misses the next one. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

A declared pattern in `statuses.pending` makes every matching value read as pending rather than as a status; a value matching no entry and no pattern is still reported; a fixture covers a pattern and a whole value together and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Built: `statuses.pending` accepts the structured entry `{pattern: "..."}` beside whole values (`SPEC-3` 0.39, `SPEC-2` 0.52); an invalid, blank or repeated pattern is a load error. Exercised by this repository's ADR type (a `TBD` / `to be decided` family) and `placeholder_vocabulary_integration.rs`. **Why:** the milestone's Done means. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged on `main`: `statuses.pending` accepts `{pattern: ...}` entries. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-177`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
