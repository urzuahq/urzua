---
Stable-Id: 01M7FVYJNPTR5CY0XQ8AJ8YZK5
Status: Planned
Phase: '1'
Track: code-health
Blocked-on: null
---
# 242 — Rule ids, variants and record types are typed

## What

Replace the string rule ids, finding variants and record type names with types generated from the rule table and the config, so a misspelt id is a compile error.

## Why

the audit of 2026-11-15 found `Finding.rule` as a string, 95 variant literals tied to the table only by tests, and record type names as plain strings at 58 sites.

## Done means

- A rule id or variant that is not in the rule table does not compile.
- The JSON report is byte-for-byte unchanged, shown by the existing golden tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
