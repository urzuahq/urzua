---
Status: Planned
Stable-Id: 01M1YN89CZE1PPE925SH2EZ7Q8
Phase: '1'
Track: accountability-identity
Blocked-on: —
---
# 55 — Build the roles and self-ack enforcement rule family

## What

Build the roles + self-ack rule family SPEC-2 already specifies: a Reviewer/Decider self-acknowledgement check (the same person can't be both author and sole approver), plus the blank/placeholder/pending distinction `field.quality` already applies to other fields, applied specifically to role fields.

## Why

SPEC-1, SPEC-2, and SPEC-3 all already reference self-ack/role semantics as part of the design, but no rule in `rules.rs` implements it and no milestone tracks building it -- specified, never routed to actual work.

## Done means

A rule family reports a record whose sole Reviewer or Decider is its Author (self-acknowledgement) and applies the blank, placeholder and pending distinction of `field.quality` to role fields, declared per type in `.urzua/config.yaml`; a planted self-acknowledged record is reported and one with an independent approver is not, and the family is exercised in urzua's own config and ADRs; the requirement of Deciders on an accepted record is carried by `MILE-132` and not repeated here.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Reshaped by `ADR-53`. **Why:** filed as a "rule family" to build in `rules.rs`. Rules are configuration now, so the engine-side work is whatever functions the roles and self-ack checks need that do not yet exist -- the family itself is config. Its real dependency is the function vocabulary, which `ADR-53` deliberately left open. | **substantive** |
> | 2026-09-18 | `Blocked-on` now names `MILE-98`. **Why:** the declared document model had no milestone -- `MILE-4` was marked absorbed into `RFC-33` and the work moved into an RFC, so six records were blocked on something the plan did not track. Naming it makes the dependency resolvable, and `narrative-field.stale` can report when it moves. | **structural** |
> | 2026-11-02 | Unblocked: `RFC-33` was Accepted; maintainer decision 2026-11-02. `Blocked-on` now reads `MILE-98`; `Status` stays `Planned`. **Why:** `narrative-field.stale` flagged the reference once `RFC-33` reached a terminal status, and the maintainer decided the dependency no longer holds. `MILE-98` is still open and stays named. | **substantive** |
> | 2026-11-04 | Re-phased from Phase 0 to Phase 1; `Status` unchanged. The part of this milestone that requires Deciders on an accepted record is carried by `MILE-132` (status-dependent required fields) and stays Phase 0; what remains here, the self-acknowledgement and roles family, is not needed to replace the adopter's linter, which has its own self-review check switched off. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the exit is the adopter's linter deleted and urzua's own repo clean in CI, and this work is on neither path, so it moves out of Phase 0. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-04 | `Blocked-on` cleared: `MILE-98` is Done (declared `identity`, `fields` and `sections`, including `###` headings read through `sections.depth`). `Status` unchanged. **Why:** `narrative-field.stale` flagged a blocker that reached a terminal status; the dependency no longer holds. | **substantive** |
