---
Stable-Id: 01M6DEN9B38Y4CFPV1W98X0WRE
Status: Planned
Phase: '0'
Track: governance-process
---
# 133 — A gate that holds an implementation change until a decision record for it is Accepted

## What

A gate that reports a change to implementation paths (for example `rust/`) whose change set names no decision record, or names one that is not yet Accepted, so implementation of an RFC's proposal cannot land ahead of its decision.

## Why

`AGENTS.md` already requires that building part of an RFC gets its own ADR, and that Status is a human decision, but nothing checks it, so the rule depends on memory. This repository builds itself with agents; a check that holds the change is what keeps the bootstrap honest. Belongs in Phase 0 because the remaining Phase 0 engine work is exactly the work it would govern.

## References

- `AGENTS.md` -- the decision-before-implementation rule this enforces.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** A gap the Phase 0 roadmap found with no record. | **substantive** |
