---
Stable-Id: 01M6DEN8ZYA5Z4DVW6E7SA10F3
Status: Planned
Phase: '0'
Track: schema-governance
---
# 132 — Declared state machines per record type

## What

A record type declares its status field as a state machine in config: the states, which are initial
and which terminal, the allowed transitions, and the fields or sections each state requires. Rules
refer to the declaration ("terminal", "initial") instead of copying status lists, and the engine
reports:

- a status value the type does not declare;
- a field a state requires but the record lacks, including Deciders on an accepted record;
- a field a state forbids but the record carries (for example an acceptance date on a record that is
  not accepted).

This milestone is the status vocabulary and the status-dependent required and forbidden fields, and
stays Phase 0. Declared transitions, unreachable states, the derived `blocked` value and computed
lifecycles such as the embodiment tier are split out to `MILE-144` (Phase 1).

## Why

No engine code validates a status value: `Error::UnknownStatus` exists in `lib.rs` and nothing
produces it. Each type's vocabulary lives only in its spec's prose, and the set of finished statuses
is copied by hand into rule options, where the copies already disagree (`BUG-178`). Records already
use values no spec declares (`Not a bug` on two bugs), the embodiment tier `Inactive` is declared but
cannot be produced, and status-dependent requirements (a real regression test on a Fixed bug, a
blocker on a Blocked milestone) are prose that nothing checks. Which statuses a type has is
repository configuration, not engine policy: this repository's bug workflow is one configuration of
the primitive.

## Done means

A type declares its statuses in config and `check` reports an undeclared status value; a state can
declare required and forbidden fields, a record violating either is reported, and a planted accepted
record without Deciders fails while a Proposed one passes; the hand-copied terminal and closed status
lists are read from the declaration instead (`BUG-178`).

## References

- `BUG-178` -- the duplicated, disagreeing status lists.
- `ADR-64` -- structured in, structured out; derived values are not hand-written.
- `RFC-48` -- the derived `blocked` value, now carried by `MILE-144`.
- `MILE-144` -- the split-out half: transitions, unreachable states and the derived `blocked` value.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** A gap the Phase 0 roadmap found with no record. | **substantive** |
> | 2026-11-02 | Re-scoped from declared status values to declared state machines. **Why:** The implicit state machine inventory found twelve undeclared lifecycles and disagreeing copies; author asked for declared state machines. | **substantive** |
> | 2026-11-04 | Split: status vocabulary and status-dependent required and forbidden fields (including Deciders on accepted) stay here at Phase 0; transitions, unreachable states, the derived `blocked` value and computed lifecycles moved to `MILE-144` at Phase 1, which this record now points at. Added Done-means. `Status` unchanged. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the adopter's linter needs a status enum and per-status required and forbidden fields, and does not need transitions or derived values. | **substantive** |
