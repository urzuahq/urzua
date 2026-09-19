---
Status: Planned
Stable-Id: 01M2W1994CH6D121YBRPAD620R
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: MILE-100
---
# 102 — Replace a hand-written linter outright, with that linter deleted

## What

`SPEC-1`'s third success criterion, and the one it calls **the real bar**:

> It replaces the hand-written linter in at least one codebase outright, with that linter deleted.
>
> Criterion 3 is the real bar. A tool that runs *alongside* the thing it was meant to replace has not
> replaced it.

## Why it is filed late

It had no milestone, and was then **deleted** from `SPEC-1` while narrowing that spec -- in the change
whose stated purpose was filing the untracked exit criteria. Two of four were dropped instead. Caught
by review.

That is worth recording rather than quietly correcting: a criterion with nothing tracking it is
invisible, and the thing most likely to happen to an invisible criterion is that someone tidies it
away.

## What "outright" means

Not "urzua also runs". The existing linter is removed from that repository, and whatever it checked is
either expressed in `.urzua/config.yaml` or consciously dropped with the reason recorded. A rule that
cannot be expressed is a finding about the schema (`MILE-51`'s standard), not a reason to keep both
tools.

## Blocked on `MILE-100`

There is no second codebase yet, so there is no hand-written linter to replace. This criterion is
strictly downstream of that one.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** `SPEC-1`'s criterion 3, which that spec calls the real bar, had no milestone and was then deleted from the spec by the change meant to file its untracked criteria. Recovered from `main` and filed. | **substantive** |
