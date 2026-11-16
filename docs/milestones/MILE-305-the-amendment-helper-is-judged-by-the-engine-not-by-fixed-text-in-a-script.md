---
Stable-Id: 01M7HY937B73AE7Q056AKADGGC
Status: Planned
Phase: '1'
Track: write-path
Blocked-on: null
---
# 305 — The amendment helper is judged by the engine, not by fixed text in a script

## What

Move the check that an amendment only adds a section and log rows out of the shell helper's fixed patterns and into the engine, reading the record type's declared log marker and a declared amendment heading.

## Why

An audit on 2026-11-16 read all non-test engine code for places that name a particular record type, status, field or workflow. It found the helper script refusing unless it sees a heading literally named "Amendment" and a revision-log row written as a blockquote table, ignoring what the record type declares.

## Done means

- The helper asks the engine whether an edit is an allowed amendment, and the engine reads the type's declared revision-log shape and amendment heading.
- A type that declares another heading and a plain-table log can be amended through the helper, shown by a test.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from the audit of 2026-11-16, asked for after the maintainer said the tooling has no opinion about which record types a repository defines and that this is driven by configuration, not by the code. | **structural** |
