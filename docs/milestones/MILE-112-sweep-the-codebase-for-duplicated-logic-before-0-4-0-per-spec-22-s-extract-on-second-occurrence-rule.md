---
Stable-Id: 01M36T2JPBG4TDMVQ1222N9EKJ
Status: Planned
Phase: '1'
Track: release-process
Implements: SPEC-22
---
# 112 — sweep the codebase for duplicated logic before 0.4.0, per SPEC-22's extract-on-second-occurrence rule

## What

A deliberate, one-time audit of `rules.rs`, `config.rs`, `check.rs`, `discovery.rs`, `graph.rs`, and
`property.rs` for logic restated in two or more places, applying `SPEC-22`'s tightened rule
retroactively rather than only going forward. Each real instance found gets the same triage this
project's own review loop already uses: fix it in the same PR if it's mechanical, or file it (as
`BUG-133`/`BUG-134`/`BUG-135` already were) if it needs a real refactor decision. Not a rewrite --
this milestone closes once the sweep has happened and every instance found has an outcome on record,
not once every instance is fixed.

## Why

`SPEC-22`'s duplication rule changed from reactive ("extract after a real bug") to proactive
("extract on the second occurrence") specifically because the reactive version kept failing to catch
its own next instance -- `BUG-105`/`106` motivated the original rule, and the same review cycle that
produced this session's fixes (`census`/`census_records`, `Config::sorted_type_names`,
`resolve_inside_repo`, `has_no_header`, `claim_status_agreement_setting`) also found two *more*
instances of the identical shape and had to file them rather than fix them on the spot (`BUG-133`,
`BUG-134`, on top of `BUG-135`'s related git-subprocess-caching gap).

Every instance found so far was found incidentally, as a side effect of a `/code-review` pass
scoped to something else. Nothing has yet looked at the codebase specifically for this pattern.
Doing that once, deliberately, before 0.4.0 ships, is cheaper than finding the next instance the same
accidental way after release -- and gives the new rule in `SPEC-22` a real first test: if a
systematic sweep still finds instances the informal review passes missed, that's evidence the rule
needs a mechanical backstop (a lint), not just a stated convention.

## References

- `SPEC-22` -- the convention this milestone applies retroactively.
- `BUG-105`, `BUG-106`, `BUG-114`, `BUG-133`, `BUG-134`, `BUG-135` -- every known instance so far,
  all found incidentally.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed. **Why:** the user asked directly for this recurring pattern to be cleaned up before 0.4.0 ships, after `SPEC-22`'s duplication rule was tightened from reactive to proactive in the same conversation. | **substantive** |
