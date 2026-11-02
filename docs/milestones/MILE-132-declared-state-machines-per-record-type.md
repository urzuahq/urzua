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
- a field or section a state requires but the record lacks;
- a declared state that nothing can reach, or that no record uses (informational);
- a transition not allowed by the declaration, between the record's committed status and its status
  in the change under check;
- a hand-written status that a derived value contradicts (for example a written `Blocked` while the
  derived `blocked` value from `RFC-48`'s sequencing relation is false). This is an error.

The same primitive covers computed lifecycles such as the embodiment tier, whose order is a
hard-coded `if` chain in `rules.rs` today.

## Why

No engine code validates a status value: `Error::UnknownStatus` exists in `lib.rs` and nothing
produces it. Each type's vocabulary lives only in its spec's prose, and the set of finished statuses
is copied by hand into rule options, where the copies already disagree (`BUG-178`). Records already
use values no spec declares (`Not a bug` on two bugs), the embodiment tier `Inactive` is declared but
cannot be produced, and status-dependent requirements (a real regression test on a Fixed bug, a
blocker on a Blocked milestone) are prose that nothing checks. Which statuses a type has is
repository configuration, not engine policy: this repository's bug workflow is one configuration of
the primitive.

## References

- `BUG-178` -- the duplicated, disagreeing status lists.
- `ADR-64` -- structured in, structured out; derived values are not hand-written.
- `RFC-48` -- the derived `blocked` value.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** A gap the Phase 0 roadmap found with no record. | **substantive** |
> | 2026-11-02 | Re-scoped from declared status values to declared state machines. **Why:** The implicit state machine inventory found twelve undeclared lifecycles and disagreeing copies; author asked for declared state machines. | **substantive** |
