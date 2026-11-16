---
Stable-Id: 01M7GX8HYC1E5HM3380T7TS1RW
Status: Planned
Phase: '1'
Track: write-path
Blocked-on: null
---
# 275 — A drafted record can carry its relation fields

## What

Let a record drafted through `urzua new` and the drafting commands state the fields that link it to other records (`Implements`, `Depends-on`, `Derives-from`) at creation.

## Why

Filing 56 milestones on 2026-11-15, the drafting helper refused any header field the scaffold had not written, so none could say which record it implements; each cites its source in prose, where no relation rule reads it.

## Done means

- `urzua new` accepts a declared relation field and its target, writes it into the header, and refuses a target that does not resolve.
- The drafting helper's finish step accepts a relation field added that way.
- The generated drafting commands tell the agent how to state what a record implements.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Found while filing the milestones of 2026-11-15; the maintainer asked that UX gaps reported by agents be tracked. | **structural** |
