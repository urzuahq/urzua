# 71 — Add a terminal status for deferred or abandoned milestones

> Status: Planned
> Stable-Id: 01M1YS1MJX3Z91EJ5MXZ1XEGQA
> Phase: 1
> Track: schema-governance
> Implements: ADR-34

## What

Extend milestone's `Status` enum (`Planned | InProgress | Blocked | Done`, ADR-34/SPEC-6) with a
terminal state for a milestone that's deliberately deprioritized or decided against, distinct from
`Blocked`'s "waiting on something specific." `bug` already has `WontFix`; `adr` already has
`Rejected`/`Superseded`. Also worth deciding alongside it: whether any record type's `Status`
should be validated against a closed set at all, since nothing in `urzua-core` currently checks
`Status` against any enum for any type. Requires an ADR-34 amendment, not a silent schema change.

## Why

Found by red-teaming a real Planned milestone (MILE-2) against "what if we decide not to do this
after all." No documented value fits: `Blocked` implies a nameable blocker, `Planned` left in place
misrepresents a reversed decision as still intended, and there's no `Superseded`-style pointer to a
replacement either. This is new capability for the state machine, not broken behavior -- nothing
crashes or emits wrong output today, the enum is just incomplete for a real case.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
