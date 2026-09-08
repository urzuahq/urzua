# 83 — A staleness drift rule for Blocked-on and similar prose pointers

> Status: Done
> Stable-Id: 01M1Z8QDN72FCDTD839NSCEYBC
> Phase: 1
> Track: schema-governance
> Implements: ADR-42
> Blocked-on: —

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

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Fixed and shipped (ADR-42): `Blocked-on` moved from a body section into `milestone`'s header (all 84 files migrated); `pointer.resolution` gained `Blocked-on` for dangling-reference checking; a new `blocked_on_stale` rule flags a resolved `Blocked-on` reference whose target reached a terminal status. **Why:** design review found `Blocked on` was the one field-shaped exception across the entire schema still living as unchecked prose, not just this one field's problem in isolation -- moving it into the header was more consistent than building a second, body-text-specific checking path. Verified against the real, live MILE-2/3 case: the rule was confirmed firing before the actual fix was applied, not only against synthetic fixtures. | **substantive** |
