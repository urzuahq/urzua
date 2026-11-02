---
Stable-Id: 01M6DFQKHEX940BY1Y6ZHGZJHX
Status: Draft
Date: 2026-11-02
Author: beauwilliams
---
# 48 — The roadmap is computed from records

## Summary

Make the order of work a computed view over records, the way `audit` is a computed view over
findings. Records already carry the inputs (`Status`, `Phase`, `Track`, `Parent`, `Blocked-on`,
`Implements`); the engine never turns them into an order. This proposes three things: a declared
sequencing relation with cycle detection (`MILE-27`), an `urzua roadmap` command that derives waves,
the ready set and the critical path from that relation, and a small set of rules that keep the
relation honest. All three are engine features driven by config, usable by any adopter; this
repository is the first user.

## Motivation

`ADR-34` deferred both pieces with explicit triggers. The sequencing relation waited for "a real case
needs it"; the generated roadmap waited for "enough milestones for a generated view to earn its cost".
Both triggers have now fired:

- **The real case.** Planning Phase 0's remaining work needed an order that `Phase`/`Track` cannot
  express. Seven parallel lanes all edit `rules.rs`, so they wait on `MILE-131`; the switchover
  (`MILE-102`) waits on `MILE-120` and `MILE-98`; six decisions gate whole lanes. That order was
  worked out by reading records and written into a hand-built HTML page. It is already the drift
  `ADR-34` warned about: the page is a second document that nothing checks against the records.
- **The scale.** There are 134 milestones. `urzua check docs/milestones/` lists findings; it does not
  say what can start now.
- **The data is there but thin.** 124 records carry `Blocked-on:`. 96 of them hold the `—`
  placeholder, and only 23 cite a record ID; the rest are prose ("a real external adopter asking for
  it"). `urzua graph` does emit `Blocked-on` and `Parent` edges, but as `narrative` edges with no
  direction semantics and no cycle check, so nothing can be ordered from them.

Separately, the pattern of epics, sub-issues, dependency links and board stages computed from state is
common practice for engineering roadmaps kept in an issue tracker. The gap that practice leaves is
that the plan lives outside the decision records it implements, so a milestone can reach `Done`
while the board still shows it blocked, and the reverse. Computing the roadmap from the records closes
that gap by construction.

## Proposal

### 1. A declared sequencing relation (`MILE-27`)

A type declares which of its fields is a sequencing relation, using the relation declarations
proposed in `RFC-47` (field, direction, inverse name). For this repository: `Blocked-on` on
`milestone` and `bug`, direction "this record waits on the target"; `Parent` as containment.

- A sequencing value is a comma-separated list of record IDs. Prose stays legal in the field only
  through an explicit marker (for example `external: a real adopter asking for it`), so a blocker
  that is not a record is still visible and is never silently dropped from the order.
- The inverse (`Blocks`) is computed, never written, so there is nothing to keep reciprocal by hand.

### 2. `urzua roadmap`

A read-only command that computes, for a scope (default: every type with a sequencing relation):

- **Waves**: topological layers over open records; wave 0 is everything with no open blocker.
- **Ready set**: open records whose blockers are all in a terminal status.
- **Critical path**: the longest chain of open blockers.
- **Per record**: what it blocks, transitively.
- **Grouping** by `Phase` then `Track` (fields named in config, not hard-coded).

Output is JSON by default, and part of `export --format json` (`MILE-115`), so a dashboard or
roadmap page is generated from it rather than hand-written. Like `audit`, nothing is stored.

### 3. Rules

- `sequence.cycle`: a cycle in a sequencing relation. Error.
- `sequence.terminal-parent-open-child`: a parent in a terminal status with a child that is not.
- `sequence.phase-order`: a record blocked on a record in a later phase.
- `sequence.unresolved-blocker`: a sequencing value that is neither a resolving ID nor marked
  external.
- The existing `narrative-field.stale` / `pointer.target-status` behavior covers a blocker that has
  since reached a terminal status.

### 4. This repository's workflow

- Re-record the order the hand-built Phase 0 roadmap holds as `Blocked-on` and `Parent` values, so
  the records carry it.
- Generate the roadmap page from `urzua roadmap` output, the same way the ledger page is generated
  from records, and retire the hand-built page.

## Open questions

1. Is `Blocked-on` itself the sequencing relation, or a separate `Depends-on` field, with
   `Blocked-on` staying a free-text narrative? Reusing it keeps one field and migrates 23 ID values;
   a new field keeps the prose meaning intact.
2. Does `Status: Blocked` become derived (an open blocker exists) or stay a human decision that a
   rule checks against the relation? `AGENTS.md` treats Status as a human decision, which argues for
   checking, not deriving.
3. Is a lifecycle stage per milestone (define, design, build, launch, operate) a status vocabulary
   (`MILE-132`) or a separate field? The roadmap can group by either; the choice belongs to `MILE-132`.
4. Does the roadmap need evidence of progress (an open change implementing a record) from outside the
   repository? That would be the first input the engine reads from a forge, and is excluded below
   unless this question decides otherwise.

## Non-goals

- Reading issue trackers or pull requests. The roadmap is computed from records in the repository.
- Scheduling, estimates, dates or assignment.
- Storing the computed order anywhere, or a hand-edited board.

## References

- `ADR-34` -- deferred both pieces with the triggers this cites.
- `SPEC-6` -- the milestone schema, whose "deliberately not built" section names both.
- `MILE-27` -- the sequencing relation this builds.
- `RFC-47` -- relation declarations with direction; a prerequisite.
- `MILE-115` -- `export --format json`, the data feed.
- `MILE-131`, `MILE-132` -- the rules split and status vocabulary this interacts with.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Drafted. **Why:** Planning Phase 0 needed an order the records could not compute, which is the trigger `ADR-34` set for both deferred pieces. | **substantive** |
