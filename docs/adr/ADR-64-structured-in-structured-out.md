---
Stable-Id: 01M6DGQK2J4AW7QRSB486Q7THH
Status: Accepted
Date: 2026-11-02
Author: beauwilliams
Deciders: beauwilliams
---
# 64 — Structured in, structured out

## Context

The engine already reads declared fields and writes JSON: every command prints a machine-readable
result, and rules read header fields named in `.urzua/config.yaml`. The gaps are the places where a
value the engine is expected to act on is free text:

- `Blocked-on` on milestones and bugs. Of 124 records carrying it, 96 hold a `—` placeholder, 23 cite
  record IDs, and the rest are prose such as "a decision to actually build it" or "a real external
  adopter asking for it". The prose values cannot be resolved, ordered or checked for staleness, so
  the engine cannot compute a roadmap from them (`RFC-48`).
- `narrative_fields` exists precisely to tolerate prose in a field that rules also read.
- A `Status` value is read from a header by several rules, but the set of legal values is repeated
  across rule options (`terminal_statuses`, `closed_statuses`) rather than declared once (`MILE-132`).

Each prose escape hatch was added for a good local reason, but each one is a place where a rule
silently examines nothing, which `SPEC-1`'s no-silent-no-op rule forbids in spirit.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Structured in and out by default; prose only with a stated reason | Every value a rule reads can be resolved, ordered and checked; output stays machine-readable | Some migration; authors must create records for conditions they used to describe in a sentence |
| B. Keep allowing prose where convenient | No migration | Rules keep examining nothing on prose values; a roadmap or state machine cannot be computed |
| C. Parse IDs out of prose | Keeps authors' wording | Guessing; the same silent misses at the edges, and two meanings in one field |

## Decision

In the context of an engine whose rules and views are computed from record headers, facing fields
whose prose values no rule can resolve, we decided that anything the engine is expected to order,
check or report on is structured on the way in (a declared field holding record IDs or values from a
declared vocabulary) and on the way out (JSON from every command), to achieve rules and views that
never silently examine nothing, accepting that conditions once written as a sentence must become
records and that existing prose fields need a migration.

Concretely:

1. By default, a field that a rule reads holds record IDs or declared values. A condition that is not
   yet a record (a pending decision, an outside event) becomes one. A prose-tolerant header field is
   an explicit exception: the change that adds it states its reason, and the field follows the
   `narrative_fields` contract (presence and shape checks, plus optional resolution of any record
   references it contains and their staleness).
2. Structure lives in headers; prose lives in sections. A rule may require a section to exist and be
   non-empty, but never parses its meaning. A new prose-tolerant header field read by a rule needs a
   stated reason in the change that adds it.
3. Every command's output is JSON. `explain` and `doctor` already comply; human-readable text, where
   wanted, is a field in the JSON or a rendering outside the engine.
4. Derived values (for example "blocked", computed from a sequencing relation) are computed, not
   written by hand. A hand-written value that a derived value contradicts is an error.
5. Configuration is held to the same rule: `.urzua/config.yaml` is validated against a declared
   schema, and values that name statuses, fields or types must resolve to declarations.

## Reversibility

Easy to relax for a single field: declare it prose-tolerant with a reason. Expensive to reverse
wholesale once rules and views depend on structured values, which is the point.

## Consequences

- Once this ADR and `RFC-48` are accepted and the migration lands, sequencing in this repository
  moves to the structured `Depends-on` field. `RFC-48` sets its contract: an optional pointer field
  whose absence means no dependencies, whose every value must resolve to a record
  (`sequence.unresolved-blocker`), and which may not form a cycle (`sequence.cycle`). A blocking fact
  that is not yet a record becomes one.
- `Blocked-on` stays, as a prose note beside `Depends-on`: it is the stated exception item 1 allows,
  because no rule orders or computes anything from it.
- `narrative_fields` stays an engine feature, and a review of any new prose field will cite this ADR.
- Status vocabularies and other value sets move toward one declaration per type (`MILE-132`), which
  is also the precondition for declared state machines.
- Writing a record takes slightly more effort: a blocking condition is a record, not a sentence.

## References

- `RFC-48` -- the computed roadmap and the `Depends-on` relation.
- `MILE-132` -- declared status values and status-dependent fields.
- `SPEC-1` -- the no-silent-no-op rule.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Drafted as Proposed. **Why:** Author stated the principle while settling `RFC-48`. | **substantive** |
> | 2026-11-02 | Headers structured and sections prose; contradicted hand-written values are errors; config held to the same rule; JSON-only output. **Why:** Author's answers to the draft's open points. | **substantive** |
> | 2026-11-03 | Prose-tolerant fields defined as the stated exception under the `narrative_fields` contract; Consequences reworded as pending and aligned with `RFC-48`: `Blocked-on` stays as a prose note beside `Depends-on`, whose contract is stated. **Why:** Review findings; the earlier wording retired `Blocked-on`, contradicting the decision that both fields exist. | **substantive** |
> | 2026-11-04 | `Proposed` -> `Accepted`. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
