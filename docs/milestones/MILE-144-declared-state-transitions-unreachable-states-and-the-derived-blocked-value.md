---
Stable-Id: 01M6HZ0VVE907G809JG5R61PH8
Status: Planned
Phase: '1'
Track: schema-governance
Implements: RFC-48
---
# 144 — Declared state transitions, unreachable states and the derived blocked value

## What

The second half of declared state machines, split from `MILE-132`: a type's declaration also carries
the allowed transitions, and the engine reports:

- a declared state that nothing can reach, or that no record uses (informational);
- a transition not allowed by the declaration, between the record's committed status and its status
  in the change under check;
- a hand-written status that a derived value contradicts (for example a written `Blocked` while the
  derived `blocked` value from `RFC-48`'s sequencing relation is false). This is an error.

The same primitive covers computed lifecycles such as the embodiment tier, whose order is a
hard-coded `if` chain in `rules.rs` today.

## Why

`MILE-132` declares the vocabulary and what each state requires. Transitions and derived values need a
committed-versus-changed comparison and the sequencing relation, which is more machinery than the
adopter's linter uses: it checks a status enum and per-status fields, and states its lifecycle only in
prose. Splitting keeps the Phase 0 exit on the smaller piece.

## Done means

An undeclared transition between a committed and a changed status is reported; an unreachable declared
state is reported as informational; a written `Blocked` contradicted by the derived value is an error;
the embodiment tier order is read from a declaration instead of a hard-coded chain.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed at Phase 1, split from `MILE-132`. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the adopter's linter needs only the status vocabulary and status-dependent fields, which stay in `MILE-132` at Phase 0, so transitions, unreachable states and derived blocked move out of the exit. | **substantive** |
