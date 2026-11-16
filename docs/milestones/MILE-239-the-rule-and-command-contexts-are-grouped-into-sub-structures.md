---
Stable-Id: 01M7FVXTZWYC5E8ZMXTDSN7N96
Status: Planned
Phase: '1'
Track: code-health
Blocked-on: null
---
# 239 — The rule and command contexts are grouped into sub-structures

## What

Group the 43-field rule context and the 39-field command context into a few named parts (diff inputs, sources, clocks, probes), and replace long parameter lists with those parts.

## Why

the audit of 2026-11-15 found three structs with 39 to 50 fields, and functions with seven to nine parameters where the function-length ratchet pushed complexity into arguments.

## Done means

- Neither context has more than a stated number of direct fields, enforced by a test.
- No function outside tests carries an allowance for too many arguments.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
