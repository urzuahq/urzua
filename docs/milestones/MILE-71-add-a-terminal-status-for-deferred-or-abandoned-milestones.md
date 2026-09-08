---
Status: Done
Stable-Id: 01M1YS1MJX3Z91EJ5MXZ1XEGQA
Phase: '0'
Track: schema-governance
Implements: ADR-34, SPEC-6
Blocked-on: —
---
# 71 — Add a terminal status for deferred or abandoned milestones

## What

Extend milestone's `Status` enum (`Planned | InProgress | Blocked | Done`, ADR-34/SPEC-6) with a
terminal state for a milestone that's deliberately deprioritized or decided against, distinct from
`Blocked`'s "waiting on something specific." `bug` already has `WontFix`; `adr` already has
`Rejected`/`Superseded`. Whether any record type's `Status` should be validated against a closed
set at all is adjacent scope, not decided here (see MILE-55).

## Why

Found by red-teaming a real Planned milestone (MILE-2) against "what if we decide not to do this
after all." No documented value fits: `Blocked` implies a nameable blocker, `Planned` left in place
misrepresents a reversed decision as still intended, and there's no `Superseded`-style pointer to a
replacement either. This is new capability for the state machine, not broken behavior -- nothing
crashes or emits wrong output today, the enum is just incomplete for a real case.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Fixed and shipped: `WontDo` added via ADR-34 amendment + SPEC-6 v0.2, template updated. **Why:** every other record type in this corpus already has a terminal "decided against" state (`bug`'s `WontFix`, `adr`'s `Rejected`/`Superseded`); milestone didn't, so a deferred or abandoned milestone had no honest representation. | **substantive** |
