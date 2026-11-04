---
Status: Planned
Stable-Id: 01M1Y5JTH1RN024BFKVMV0Q6XF
Phase: '0'
Track: review-workflow
Implements: —
---
# 27 — Depends-on/Blocked-by relationship with cycle detection

## What

A genuine sequencing relationship beyond Phase/Track tags, checked for resolution and cycles.

## Why

Named explicitly in ADR-34/SPEC-6 as deferred until a real case needs it -- Phase/Track tags are sufficient for the backlog as it exists today.

## Done means

`RFC-48` section 1 is built, and no more of `RFC-48` than that. A `relation_pairs` entry declares `Depends-on` and its inverse `Blocks` as a sequencing pair with an `acyclic` key; `Depends-on` is a pointer field. `check` reports a cycle in an acyclic pair (a planted-cycle fixture, including a self-reference and a cycle across record types) and a `Depends-on` value that resolves to no record (a dangling-target fixture), and a pair declared `reciprocity: required` reports a missing inverse through the existing reciprocity rule. A declared acyclic pair that examines no claims is reported, not a silent pass. The behaviour is documented in `SPEC-3`. It is exercised in this repository: `.urzua/config.yaml` declares the pair and at least one committed milestone carries a `Depends-on` value that `check` examines. Out of scope, owned elsewhere: `urzua roadmap` (`MILE-182`), the derived `blocked` value (`MILE-144`), the `condition` record type and the other `sequence.*` rules of `RFC-48`; migrating the existing `Blocked-on` values is a separate decision made after this lands.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Phase` `1` -> `0`; Done means narrowed to the sequencing relation, its cycle and dangling-target findings and their use in this repository. **Why:** Maintainer decision, 2026-11-04: `MILE-27` is pulled into Phase 0 because `MILE-182` (`urzua roadmap`) and the derived `blocked` value of `MILE-144` wait on it. The earlier Done means said "`RFC-48`'s design is what is built", which would have absorbed the roadmap command, conditions and derived values that `MILE-182` and `MILE-144` own. | **substantive** |
> | 2026-11-04 | `Blocked-on: RFC-48` removed. **Why:** `RFC-48` is accepted; this milestone is buildable. | **substantive** |
> | 2026-11-02 | `Blocked-on` moved from the trigger to `RFC-48`. **Why:** The trigger fired: planning Phase 0 needed an order `Phase`/`Track` could not express; `RFC-48` proposes the design. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
