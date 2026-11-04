---
Status: Planned
Stable-Id: 01M1Y5JYW47SYHYNYVFXDEFE12
Phase: '1'
Track: schema-governance
Implements: RFC-1
Blocked-on: —
---
# 35 — Product-level work lives in urzua records, and a new kind needs config only

## What

Ideas, customer feedback, and other pre-RFC product material are tracked as urzua records: as a
`milestone` or `bug` where they fit, and as a newly configured record type where they do not. The
deliverable is proof that a new record kind needs a `.urzua/config.yaml` change and zero engine
changes, as `milestone` and `bug` did (`ADR-34`, `ADR-35`).

The original questions remain as sub-questions: whether a short discovery gate (about an hour) should
decide if an idea earns real design time before an RFC is written, whether a resource-budget
("appetite") field type is needed, and whether a profile whose primary content is narrative rather than
structured fields is needed.

## Why

The project is moving product-level tracking out of an external issue tracker and into its own
records. If adding a kind for that material needs Rust changes, that is a defect against the
config-driven thesis, not a feature request. The discovery-gate, appetite and narrative-profile
questions still feed `RFC-1`'s open Product/PRD question; they are now answered by building the
smallest real case rather than in the abstract.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Re-scoped from exploring a discovery gate and a narrative profile to proving a new record kind needs only config, and moved from Phase 1 to Phase 0. **Why:** product-level tracking is moving into urzua's own records, so the config-only claim is now on the critical path; the original questions are kept as sub-questions. | **substantive** |
> | 2026-11-04 | Re-phased from Phase 0 to Phase 1; `Status` unchanged. Not needed to replace the adopter's linter. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the exit is the adopter's linter deleted and urzua's own repo clean in CI, and this work is on neither path, so it moves out of Phase 0. | **substantive** |
