---
Stable-Id: 01M7FVZ9W1V0R0YH4WJKN9K4G0
Status: Planned
Phase: '1'
Track: code-health
Blocked-on: null
---
# 245 — A current toolchain, the 2024 edition and a narrower public surface

## What

Bump the pinned toolchain to a current stable, move the workspace to the 2024 edition, compile each regular expression once, and make everything that need not be public crate-private.

## Why

the audit of 2026-11-15 found the toolchain pinned to a release about eighteen months old on the 2021 edition, regular expressions compiled per call at three sites, and 585 public functions in crates that are never published.

## Done means

- The pinned toolchain is no more than two stable releases behind at the time of the change, and a stated policy says when it is bumped.
- Every crate is on the 2024 edition.
- No regular expression is compiled inside a function that runs per record.
- The public item count of the core crate falls, and the number is recorded.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
