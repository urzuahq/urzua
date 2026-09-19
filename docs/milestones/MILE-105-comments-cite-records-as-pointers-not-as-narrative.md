---
Status: Planned
Stable-Id: 01M2WAN2DE9D78643ZN7RRKARC
Phase: '1'
Track: governance-process
Implements: —
Blocked-on: —
---
# 105 — Comments cite records as pointers, not as narrative

## What

**232 comment lines** in `rust/` name a record -- `BUG-43`, `RFC-35`, `MILE-98`. They are two
different things under one habit, and only one of them belongs:

| | |
|---|---|
| `//! Implements: SPEC-0001` | a structured back-pointer the engine reads and `embodiment.consistency` checks. Load-bearing, and the premise of the project |
| *"…which `BUG-43` was filed to fix, and the third form re-entered it"* | narration of how the code got here |

The second restates history a reader can get from `git blame` and the record itself, and it ages
badly: a comment describing what a superseded record decided outlives the supersession.

## Why

The global engineering convention this project works under states it directly: never narrate the
change, task, PR or ticket origin, or debugging history. **61 of the 232 were added in a single
session**, so the habit is active rather than historical.

The cost is not stylistic. A comment saying *"fixed in `BUG-43`"* is a claim about a record, and this
project has spent considerable effort establishing that claims about records should be checkable.
That one is not -- it has no structure, nothing resolves it, and it is invisible to
`embodiment.locator-exists` and every other rule.

## What to decide

Whether a bare `(BUG-43)` tag -- a pointer with no prose -- is acceptable shorthand or should also go.
Arguments both ways: it is genuinely useful for finding the reasoning, and it is exactly the kind of
unstructured reference a `Realized-by` locator exists to replace.

Not decided here. The narration is unambiguous and can be removed first.

## Scope

A sweep, not a rule. Nothing in the engine can check comment prose, and nothing should try -- this is
convention, enforced by review.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** counted while cleaning comments in a release branch -- 232 comment lines name a record, 61 of them added in one session. Structured back-pointers are the point of this project; prose about which ticket produced a line is history a reader can get from `git blame`, and it outlives the records it describes. | **substantive** |
