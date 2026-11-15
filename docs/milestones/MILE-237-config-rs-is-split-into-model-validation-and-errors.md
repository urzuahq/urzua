---
Stable-Id: 01M7FVXAVKKVQMCZMQGADXPC6C
Status: Planned
Phase: '1'
Track: code-health
Blocked-on: null
---
# 237 — config.rs is split into model, validation and errors

## What

Split `rust/crates/urzua-core/src/config.rs` into modules for the model, validation, errors and tests, with no behaviour change.

## Why

the audit of 2026-11-15 measured the file at 7,523 lines, 4,680 of them production code, mixing the model, validation and a 42-variant error enum. It is the worst file for clones and string allocation.

## Done means

- No source file in the config module exceeds a stated line ceiling, enforced by a test like the function-length ratchet.
- The public names other crates use are unchanged, and the full test suite passes with no test edited for behaviour.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
