---
Stable-Id: 01M7FVTJ4BVJY5D0AEX22774F7
Status: Planned
Phase: '1'
Track: user-docs
Blocked-on: null
---
# 225 — The README rule list is generated from the rule table

## What

Replace the README's hand-written list of checks with one generated from the rule table, so it cannot fall behind the binary.

## Why

the audit of 2026-11-15 found the README listing 8 rules as the full rule set while the binary ships 74.

## Done means

- The README's rule summary is produced by the same generator as the rule table, and `make rule-table-check` fails when it is out of date.
- The README states the number of rules and families the build ships.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
