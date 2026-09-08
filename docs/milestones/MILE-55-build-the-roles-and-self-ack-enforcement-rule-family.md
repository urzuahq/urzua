---
Status: Planned
Stable-Id: 01M1YN89CZE1PPE925SH2EZ7Q8
Phase: '0'
Track: accountability-identity
Blocked-on: —
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
