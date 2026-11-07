---
Stable-Id: 01M6JKNPGW97WYMKFRJ94935W6
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/embodiment.rs#embodiment_consistency@sha256:abf19b57ccf88e987c32a44cec011e5c78fd5c59062f6b534ca1dc4929820d8f, test:rust/crates/urzua-cli/tests/test_tier_integration.rs#a_test_pointer_verifies_and_a_code_pointer_alone_does_not@sha256:b89acfd47fe19792744041288825020baf8894d3ceca6d2c5ff5bb21ef9f01dc, test:rust/crates/urzua-cli/tests/back_pointer_integration.rs#without_a_comment_the_stated_tier_disagrees@sha256:5a2633630df49c976fde2328fdf2270c4368b284c816a89b003369327a8b3183, test:rust/crates/urzua-core/src/rules/embodiment.rs#a_record_stating_no_tier_with_no_evidence_stays_unjudged_and_unexamined@sha256:d2d85033ab327907c6a6205bd9b94917e45e187add79caede8fdeb8d939da674, test:rust/crates/urzua-core/src/rules/embodiment.rs#a_finding_that_used_no_back_pointer_names_only_realized_by@sha256:d3fb7df69b40f2b1764682d395a822a686f78785f445f7d5e2eab301c89a2d08, code:rust/crates/urzua-core/src/rules/embodiment.rs#evidence_description@sha256:07894f88672314a5348ba78214ee80e1acd8f7907df24c71c3cb6c3f343c7a2d, code:.urzua/config.yaml#rules::embodiment.consistency@sha256:2c6819d6375e3b4f59b879cd38d1a3b458329447e047bbb64ab06de0a1240886, code:.urzua/config.yaml#rules::embodiment.back-pointer-resolution@sha256:be8df16da0185d6abd1edd784937fc9145c897eb80138620087e6bd2d3e35bd9
---
# 149 — Embodiment.consistency judges a record with no Realized-by field when back-pointers cite it

## What

Judge a record that has no `Realized-by` field when code back-pointers cite it, instead of skipping it.

## Why

The rule does not judge such a record, so the adopter would have to add an explicit empty `Realized-by` to every record in a corpus of several hundred before its code evidence counts. A field added only to make a checker look is the restated-evidence burden urzua exists to remove; a record the code cites is a record whose claim can be judged. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

- A record with cited code and no `Realized-by` field is judged. — evidence: code:rust/crates/urzua-core/src/rules/embodiment.rs#embodiment_consistency@sha256:abf19b57ccf88e987c32a44cec011e5c78fd5c59062f6b534ca1dc4929820d8f, test:rust/crates/urzua-cli/tests/test_tier_integration.rs#a_test_pointer_verifies_and_a_code_pointer_alone_does_not@sha256:b89acfd47fe19792744041288825020baf8894d3ceca6d2c5ff5bb21ef9f01dc, test:rust/crates/urzua-cli/tests/back_pointer_integration.rs#without_a_comment_the_stated_tier_disagrees@sha256:5a2633630df49c976fde2328fdf2270c4368b284c816a89b003369327a8b3183
- A record with neither cited code nor `Realized-by` is still not judged. — evidence: test:rust/crates/urzua-core/src/rules/embodiment.rs#a_record_stating_no_tier_with_no_evidence_stays_unjudged_and_unexamined@sha256:d2d85033ab327907c6a6205bd9b94917e45e187add79caede8fdeb8d939da674
- A fixture covers both, and the finding text says which evidence was used. — evidence: test:rust/crates/urzua-cli/tests/test_tier_integration.rs#a_test_pointer_verifies_and_a_code_pointer_alone_does_not@sha256:b89acfd47fe19792744041288825020baf8894d3ceca6d2c5ff5bb21ef9f01dc, test:rust/crates/urzua-core/src/rules/embodiment.rs#a_finding_that_used_no_back_pointer_names_only_realized_by@sha256:d3fb7df69b40f2b1764682d395a822a686f78785f445f7d5e2eab301c89a2d08, code:rust/crates/urzua-core/src/rules/embodiment.rs#evidence_description@sha256:07894f88672314a5348ba78214ee80e1acd8f7907df24c71c3cb6c3f343c7a2d
- The capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests. — evidence: code:.urzua/config.yaml#rules::embodiment.consistency@sha256:2c6819d6375e3b4f59b879cd38d1a3b458329447e047bbb64ab06de0a1240886, code:.urzua/config.yaml#rules::embodiment.back-pointer-resolution@sha256:be8df16da0185d6abd1edd784937fc9145c897eb80138620087e6bd2d3e35bd9

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Records with no `Realized-by` judged when cited (`SPEC-2` `0.30`): exercised here by `ADR-2`, `ADR-3`, `ADR-5` and `ADR-6`, whose crates cite them and whose stated `Not started` was corrected to the computed value; a fixture covers cited, uncited and the finding text. **Why:** the Done-when asks for it. Status unchanged; the maintainer closes. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #198: `embodiment.consistency` judges a record with no `Realized-by` when a back-pointer cites it. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
