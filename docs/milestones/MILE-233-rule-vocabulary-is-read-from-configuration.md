---
Stable-Id: 01M7FVWCQYEXD0T7VB73XWWCXF
Status: Planned
Phase: '1'
Track: schema-governance
Blocked-on: null
---
# 233 — Rule vocabulary is read from configuration

## What

Move the words rules compare against into configuration: the back-pointer markers and comment openers, the closing verbs, the Y-statement phrases, the placeholder words, and the waiver field names.

## Why

`ADR-101`'s amendment of 2026-11-15. the audit of 2026-11-15 found these vocabularies compiled in, in English, so a team with other conventions or another language cannot change them.

## Done means

- Each vocabulary has a config key whose default is today's list; a configuration that declares none behaves as before, shown by a test.
- A repository that declares another back-pointer marker and another placeholder list gets findings by its own words, shown by a test.
- `SPEC-3` lists which vocabularies stay fixed as format invariants (the pin marker, the scaffold placeholder, the override log columns) with their reasons.
- The capability is exercised in urzua's own `.urzua/config.yaml` and corpus, and `urzua check` reports 0 unwaived findings.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
