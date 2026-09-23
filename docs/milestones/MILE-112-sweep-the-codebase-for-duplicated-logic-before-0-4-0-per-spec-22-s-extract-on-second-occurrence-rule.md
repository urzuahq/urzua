---
Stable-Id: 01M36T2JPBG4TDMVQ1222N9EKJ
Status: Done
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

## Outcome

The sweep ran once, deliberately, over all six files. Found 3 new instances beyond the ones already
known from incidental review, all in `rules.rs` -- `config.rs`, `check.rs`, `discovery.rs`, `graph.rs`,
and `property.rs` had none. All 3 were mechanical (no design decision needed) and fixed in the same
pass, each with its own bug record:

- `BUG-139` -- `record_id`/`filename_number`, the same filename-id extraction one level over `BUG-133`'s
  segment-grammar fix.
- `BUG-140` -- four reference-resolving rules sharing one declared-field guard chain.
- `BUG-141` -- eight config-schema rules sharing one record-type inventory skeleton, self-acknowledged
  as "the same shape" in six of their own doc comments but never extracted.

This is a real answer to the question this milestone's own "Why" section posed: **a systematic sweep
did find instances the informal `/code-review` passes missed** (3 of them, past `SPEC-22`'s
second-occurrence threshold in one case by six occurrences). That is evidence a mechanical backstop
(a lint or a script-based duplication check) would catch what convention alone did not -- worth a
follow-up decision, not decided here.

## References

- `SPEC-22` -- the convention this milestone applies retroactively.
- `BUG-105`, `BUG-106`, `BUG-114`, `BUG-133`, `BUG-134`, `BUG-135`, `BUG-138` -- every instance known
  before this sweep, all found incidentally.
- `BUG-139`, `BUG-140`, `BUG-141` -- the instances this sweep itself found.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed. **Why:** the user asked directly for this recurring pattern to be cleaned up before 0.4.0 ships, after `SPEC-22`'s duplication rule was tightened from reactive to proactive in the same conversation. | **substantive** |
> | 2026-09-23 | Done. The sweep ran over all six files; found and fixed 3 new instances (`BUG-139`/`140`/`141`), all mechanical. Closes per this record's own stated bar: the sweep happened and every instance found has an outcome on record. | **substantive** |
