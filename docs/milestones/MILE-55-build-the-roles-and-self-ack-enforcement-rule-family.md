---
Status: Planned
Stable-Id: 01M1YN89CZE1PPE925SH2EZ7Q8
Phase: '0'
Track: accountability-identity
Blocked-on: RFC-33
---
# 55 — Build the roles and self-ack enforcement rule family

## What

Build the roles + self-ack rule family SPEC-2 already specifies: a Reviewer/Decider self-acknowledgement check (the same person can't be both author and sole approver), plus the blank/placeholder/pending distinction `field.quality` already applies to other fields, applied specifically to role fields.

## Why

SPEC-1, SPEC-2, and SPEC-3 all already reference self-ack/role semantics as part of the design, but no rule in `rules.rs` implements it and no milestone tracks building it -- specified, never routed to actual work.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Reshaped by `ADR-53`. **Why:** filed as a "rule family" to build in `rules.rs`. Rules are configuration now, so the engine-side work is whatever functions the roles and self-ack checks need that do not yet exist -- the family itself is config. Its real dependency is the function vocabulary, which `ADR-53` deliberately left open. | **substantive** |
