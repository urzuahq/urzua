---
Stable-Id: 01M7FVZH81ZWTTTQ81VR6A0WDV
Status: Planned
Phase: '1'
Track: code-health
Blocked-on: null
---
# 246 — The IO crate and the report module are split by concern

## What

Split the IO crate's single 2,500-line file into discovery, git, clock, identity, locking and hashing, remove the process-global clock, and split the module that holds every command's report type.

## Why

the audit of 2026-11-15 found the IO crate to be one file mixing seven concerns with a process-global clock, and one 2,181-line module holding every report type; the core crate's only leak is a process exit code in the report trait.

## Done means

- No source file in the IO crate or the report module exceeds the stated line ceiling.
- The clock is passed in, not read from a global, so two tests with different dates can run in one process.
- The core crate does not name a process exit code.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
