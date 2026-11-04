---
Stable-Id: 01M6DEN9B38Y4CFPV1W98X0WRE
Status: InProgress
Phase: '0'
Track: governance-process
---
# 133 — A gate that holds an implementation change until a decision record for it is Accepted

## What

A gate that reports a change to implementation paths (for example `rust/`) when the change implements a proposal that needs a design decision and its change set names no decision record, or names one that is not yet Accepted, so implementation of an RFC's proposal cannot land ahead of its decision. Mechanical bug fixes, which `AGENTS.md` lets land with a bug record, are outside the gate.

## Why

`AGENTS.md` already requires that building part of an RFC gets its own ADR, and that Status is a human decision, but nothing checks it, so the rule depends on memory. This repository builds itself with agents; a check that holds the change is what keeps the bootstrap honest. Belongs in Phase 0 because the remaining Phase 0 engine work is exactly the work it would govern.

## References

- `AGENTS.md` -- the decision-before-implementation rule this enforces.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** A gap the Phase 0 roadmap found with no record. | **substantive** |
> | 2026-11-02 | Scoped to proposal implementations. **Why:** `AGENTS.md` lets mechanical bug fixes land with only a bug record. | **substantive** |
> | 2026-11-04 | Narrowest buildable reading shipped as `embodiment.back-pointer-decision-accepted` (`SPEC-2` 0.21): a source back-pointer citing a record not in an accepted status is reported, file-granular on `--base`. Detecting a change that implements a proposal with no back-pointer, and classifying a change as a mechanical fix, remain open. **Why:** a back-pointer is the only machine-readable claim that code implements a record. | **substantive** |
> | 2026-11-04 | The gate judges a back-pointer against the cited type's declared `statuses.implementable` (falling back to the rule's `accepted_statuses`, else unjudgeable) instead of its terminal `success`; this repository declares the lists and enables the gate. `Status` unchanged. **Why:** maintainer decision of 2026-11-04: no type knowledge is hardcoded, so terminal success cannot mean implementation is allowed. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #181, #176: opt-in rule `embodiment.back-pointer-decision-accepted` judges a back-pointer against the cited type's `statuses.implementable`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | `Status` Done -> InProgress, and the partial evidence moved out of `Realized-by` into this row. **Why:** the closure audit of #234 checked the What against the code. The What asks for a gate that reports a change implementing a proposal whose change set names no decision record, or one that is not yet Accepted. Built: `embodiment.back-pointer-decision-accepted` (a back-pointer citing a record that is not implementable), with `diff.source-requires-record` (`MILE-124`) requiring some governed record in the diff. Not built, as the 2026-11-04 row above already said: detecting a change that implements a proposal and carries no back-pointer, and classifying a change as a mechanical fix. The earlier Done contradicted this record's own log. Evidence of the partial delivery, no longer claimed as complete: rust/crates/urzua-core/src/rules/back_pointer.rs, rust/crates/urzua-cli/tests/back_pointer_integration.rs. What remains is real Phase 0 scope and stays open. | **structural** |
