---
Stable-Id: 01M6JKNS1NA8ECXQDEF25RA50M
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/status.rs, test:rust/crates/urzua-cli/tests/placeholder_vocabulary_integration.rs
---
# 153 — Statuses.require treats declared placeholder vocabulary as missing

## What

Make `statuses.require` count a declared placeholder vocabulary (`pending ...`, `none recorded`, a dash, `Unknown`) as a missing value, using the type's declared `pending` vocabulary.

## Why

Today only blank or unedited template text counts as missing, so an Accepted record whose Deciders or Reviewers read `(pending ...)` or `Unknown` passes. The type already declares a `pending` vocabulary, but it does not feed this rule: a record can claim a state its own fields contradict. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

A value matching the declared placeholder vocabulary in a required field of a required status is reported; a real value that merely contains the word is not; the vocabulary is declared once and shared with the existing pending check; and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Implemented: `status.required-field-missing` counts a whole value of the type's declared `statuses.pending` as missing; this repository's `adr` type declares `pending: [pending, unknown, none recorded]` and `placeholder_vocabulary_integration.rs` runs the shipped config. `Status` stays `Planned`. **Why:** the Done-when contract; status is a human decision. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #191: `status.required-field-missing` counts the declared `statuses.pending` vocabulary as missing. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
