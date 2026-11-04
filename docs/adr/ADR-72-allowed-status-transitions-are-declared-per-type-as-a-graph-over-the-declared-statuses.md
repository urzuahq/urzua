---
Stable-Id: 01M6M6574BD7S9HVGNT8H5TTTF
Status: Proposed
Date: 2026-11-04
Author: beauwilliams
Deciders: null
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: null
---
# 72 — Allowed status transitions are declared per type as a graph over the declared statuses

## Context

`MILE-132` declared each type's status vocabulary; it did not declare which status may follow which.
`RFC-1` section 1a proposes a profile that declares "its `status` enum and legal transitions", and
`ADR-10` accepted the profile mechanism while deferring transitions. `RFC-28` ruled transition
validation out of its own scope. `MILE-144` asks the engine to report a transition the declaration
does not allow (committed status to changed status), a declared status nothing can reach, and the
embodiment tier order read from a declaration. None of this can be built without a schema for the
graph, and that schema is hard to change once adopters write it.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Per-status `to` list inside `statuses` (`transitions: {Proposed: [Accepted, Rejected], ...}`) | Reuses the existing declaration; names checked against `values`; reachability falls out of the graph | A second place to keep in step with `values` |
| B. A list of `{from, to}` edges | Easy to diff and to extend with per-edge fields | Verbose; states with many exits repeat |
| C. Leave transitions in prose | No schema | Nothing is checked; the adopter linter keeps its own copy |

## Decision

*In the context of types that already declare their statuses, facing a lifecycle stated only in
prose, we decided to add an optional `transitions` mapping to a type's `statuses` declaration (option
A): each key is a declared status and its value the declared statuses that may follow it, validated
against `values`, with `initial` naming where a new record may start. `check` compares a record's
status at the base ref with its status in the change under check, reports an undeclared transition,
and reports a declared status that is unreachable from `initial`. A type with no `transitions` is
unchanged.* The derived `blocked` value is not part of this decision; it needs the `Depends-on`
relation (`MILE-27`, `RFC-48`).

## Reversibility

Moderate. The key is optional and additive, but once adopters write graphs, renaming or reshaping it
is a config migration for each of them.

## Consequences

- A repository opting in states its lifecycle once, in config, and the engine reports departures.
- Needs the committed status, so the rule runs only under `--base`; a whole-repo run reports it as
  not applicable rather than passing.
- Terminal `success` and `failure` could later derive `claim_retained` from the graph (`MILE-144`).

## References

- `RFC-1` section 1a, `ADR-10`, `RFC-28`, `RFC-48`, `ADR-64`, `MILE-132`, `MILE-144`, `MILE-27`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Drafted as Proposed. **Why:** building `MILE-144`'s declared transitions is part of `RFC-1`'s proposal, which needs its own ADR; awaiting maintainer acceptance. | **substantive** |
