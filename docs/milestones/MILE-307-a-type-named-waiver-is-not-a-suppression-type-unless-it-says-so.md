---
Stable-Id: 01M7HY9T4BYP4MHQJKESXZ8HYK
Status: Planned
Phase: '1'
Track: schema-governance
Blocked-on: null
---
# 307 — A type named waiver is not a suppression type unless it says so

## What

Stop treating a record type as the suppression type because of its name.

## Why

An audit on 2026-11-16 read all non-test engine code for places that name a particular record type, status, field or workflow. It found that a type named `waiver` with no declared role still becomes the suppression type, which is the engine reading meaning into a name.

## Done means

- A type is a suppression type only when it declares the role; a type named `waiver` without it is an ordinary type, shown by a test.
- `urzua migrate config` or a load notice tells a repository that relied on the name to declare the role.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from the audit of 2026-11-16, asked for after the maintainer said the tooling has no opinion about which record types a repository defines and that this is driven by configuration, not by the code. | **structural** |
