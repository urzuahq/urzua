---
Stable-Id: 01M6J91RCGFRT55SHNQ2K8GQM8
Status: Accepted
Date: 2026-11-04
Accepted-on: 2026-11-04
Author: beauwilliams
Deciders: beauwilliams
---
# 68 — Implementation-permitted statuses are declared per type

## Context

`embodiment.back-pointer-decision-accepted` (`MILE-133`) holds code that cites a record
(`Implements: ADR-12`) until the record is at a status where implementation is allowed. Its first
version read "allowed" as the cited type's terminal `statuses.success`. That is a type assumption
smuggled into the engine: for an ADR the two coincide (Accepted), but a milestone is implementable
while `Planned` or `InProgress` and only reaches its terminal success (`Done`) once the code has
landed, and a bug is implementable while `Open`. Reading `success` would report every commit that
implements an open milestone. The engine's founding claim is that no type knowledge is hardcoded.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| Keep terminal `success` | No new key | Wrong for milestones and bugs; a hardcoded type assumption |
| Rule option only (`accepted_statuses`) | No schema change | One list for every cited type; cannot differ between an ADR and a milestone |
| Per-type `implementable` list in the status vocabulary | Each type states its own answer; reuses the existing declaration | One more optional key to keep in step with `values` |

## Decision

*In the context of a gate that must know when code may implement a record, facing types whose
implementable statuses differ from their terminal ones, we decided that each type declares an
optional `implementable` list in its `statuses` vocabulary, validated as a subset of `values`, to
keep the engine free of type knowledge, accepting one more declaration per type.* The gate judges a
back-pointer against the cited type's `implementable`, then the rule's `accepted_statuses`, and
otherwise reports the status as unjudgeable rather than passing it.

## Reversibility

Easy. The key is optional and additive, read by one opt-in rule; removing it returns that rule to
its fallback option.

## Consequences

- A repository enabling the gate declares `implementable` for every type its code cites.
- A type that declares none, with no `accepted_statuses`, is reported as unjudgeable on each
  citation, so a missing declaration is visible rather than silent.
- Terminal `success` keeps its other meanings (staleness, claim agreement) unchanged.

## References

- `MILE-133`, `MILE-132`, `SPEC-3`, `SPEC-2`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Status changed from Proposed to Accepted. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit dafb2cb12. | **structural** |
