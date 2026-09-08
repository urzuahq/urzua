# 83 — A staleness drift rule for Blocked-on and similar prose pointers

> Status: Planned
> Stable-Id: 01M1Z8QDN72FCDTD839NSCEYBC
> Phase: 1
> Track: schema-governance
> Implements: —

## What

Two things, decided together since the second gates the first: (1) whether `Blocked on` (and any
similar field describing a dependency in prose) should be required to cite a real record ID rather
than free text, the same way `Implements`/`Derives-from`/`Parent` already must; and (2) a drift rule
that flags a `Blocked on` pointer whose target has since reached a terminal state (`Fixed`,
`Accepted`, `Done`) as likely-stale and needing a human recheck — the same shape of problem
`embodiment.consistency` (ADR-32) already solved for `Embodiment` via git-blame-based drift
detection, applied to a different field.

## Why

Found live during a backlog triage: MILE-2 and MILE-3 both list `Blocked on: Milestone: Fix urzua
new's template-priority bug` — that bug (BUG-3) has been `Fixed` for a while, but nothing caught the
blocker going stale, because `Blocked on` is free-text prose, not a checkable pointer. If it had
been `Blocked on: BUG-3`, a drift rule could have flagged it the moment BUG-3's `Status` changed. A
milestone whose stated blocker silently resolved and nobody noticed defeats the purpose of tracking
the blocker at all.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
