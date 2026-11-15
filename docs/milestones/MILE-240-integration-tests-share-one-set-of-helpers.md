---
Stable-Id: 01M7FVY27CQQCFVVGD9NB5T40E
Status: Planned
Phase: '1'
Track: code-health
Blocked-on: null
---
# 240 — Integration tests share one set of helpers

## What

Move the repeated test helpers (`git`, `repo`, `run`, `run_urzua`) into the shared test module and delete the copies.

## Why

the audit of 2026-11-15 counted 77 test files defining their own `git` helper, 35 their own `repo`, and 50 their own runner, despite a shared module.

## Done means

- A test fails when an integration test file defines a helper the shared module provides.
- The integration test suite passes with the copies removed.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
