---
Stable-Id: 01M7FVYA9RQXC0W3JMFB55KMCM
Status: Planned
Phase: '1'
Track: code-health
Blocked-on: null
---
# 241 — check.rs is split by concern

## What

Split `rust/crates/urzua-cli/src/commands/check.rs` into scope selection, diff inputs, notices and the run, and remove the duplicated record-scope block.

## Why

the audit of 2026-11-15 measured 2,845 production lines and about 90 functions in the file, with the block that selects the records in scope copied twice.

## Done means

- No source file under the check command exceeds the stated line ceiling.
- The record-scope selection exists once.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
