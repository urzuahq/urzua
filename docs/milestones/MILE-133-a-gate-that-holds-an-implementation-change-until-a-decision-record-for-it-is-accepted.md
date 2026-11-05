---
Stable-Id: 01M6DEN9B38Y4CFPV1W98X0WRE
Status: Done
Phase: '0'
Track: governance-process
Realized-by: code:rust/crates/urzua-core/src/rules/diff_gate.rs, code:rust/crates/urzua-core/src/rules/back_pointer.rs, code:.urzua/config.yaml#rules::diff.source-requires-record@sha256:02712371f51a2c09eaf27aa70923b75bc692283a021ffe3edd21edbd8af63873, test:rust/crates/urzua-core/src/rules/diff_gate.rs, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs, test:rust/crates/urzua-cli/tests/back_pointer_integration.rs, test:rust/crates/urzua-cli/tests/acceptance.rs
---
# 133 — A gate that holds an implementation change until a decision record for it is Accepted

## What

A gate that reports a change to implementation paths (for example `rust/`) when the change implements a proposal that needs a design decision and its change set names no decision record, or names one that is not yet Accepted, so implementation of an RFC's proposal cannot land ahead of its decision. Mechanical bug fixes, which `AGENTS.md` lets land with a bug record, are outside the gate.

## Why

`AGENTS.md` already requires that building part of an RFC gets its own ADR, and that Status is a human decision, but nothing checks it, so the rule depends on memory. This repository builds itself with agents; a check that holds the change is what keeps the bootstrap honest. Belongs in Phase 0 because the remaining Phase 0 engine work is exactly the work it would govern.

## Done means

A gate reports a change to declared implementation paths when the change names no decision at a decided status, and a mechanical fix is not reported. Built: the opt-in `diff.source-requires-record` takes `decision_types` (each type with the statuses that count as decided), `fix_types` (types that exempt a change) and `link_fields` (relation fields linking a changed record to a decision, such as a milestone's `Implements`), and reports each changed source path when the diff changes none of a decision at a decided status, a record linking one, or a fix record; it is off unless declared (`MILE-124`). `embodiment.back-pointer-decision-accepted` additionally judges a source back-pointer against the cited type's `statuses.implementable`. This repository declares the gate in `.urzua/config.yaml` (adr at Accepted, spec at Accepted or Shipped, milestone linking via `Implements`, bug as the fix type), and a planted ahead-of-decision change (a source change beside a Proposed ADR) is reported by the tests in `rust/crates/urzua-cli/tests/diff_gate_integration.rs` and the `corpus-h` acceptance cases.

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
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-05 | The decision gate built as the maintainer decided on 2026-11-05: an opt-in, config-driven extension of `diff.source-requires-record` (`MILE-124`) with `decision_types`, `fix_types` and `link_fields`, not a parallel rule. A diff touching declared source paths must also change a decision at a decided status, a record whose link field names one, or a fix record; otherwise each changed source path is a finding offering the ADR-73 `add-record` action (`source-without-decision`). Detecting a change that implements a proposal with no back-pointer, and classifying a mechanical fix, are met by that rule: the first by requiring the decision in the change set rather than a back-pointer, the second by `fix_types`. Declared in `.urzua/config.yaml`. **Why:** the two items the 2026-11-04 rows left open; `SPEC-2` 0.84, `SPEC-3` 0.73, `SPEC-4` 0.8. | **substantive** |
> | 2026-11-05 | `Realized-by` added and `Status` InProgress -> Done. **Why:** Done means is met by the change above, checked against the code: the unit tests in `rules/diff_gate.rs`, the integration tests in `diff_gate_integration.rs` (including this repository's own declaration on a throwaway repository, and a Proposed ADR that does not release a source change) and the `corpus-h` gate cases. Done means was reworded to describe what was built. | **structural** |
