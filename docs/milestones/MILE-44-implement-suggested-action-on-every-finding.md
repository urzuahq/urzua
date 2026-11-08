---
Status: Done
Realized-by: code:rust/crates/urzua-core/src/report.rs, code:rust/crates/urzua-core/src/rules/action.rs, test:rust/crates/urzua-cli/tests/suggested_action_integration.rs#a_finding_carries_the_action_its_variant_declares@sha256:622ae8868e6d433479b304098a5806870a70d74d5388f726f6e0f6609e25ad61, code:rust/crates/urzua-core/src/rules/table.rs, test:rust/crates/urzua-cli/tests/suggested_action_integration.rs#rules_lists_each_variant_with_its_action_or_its_exemption_reason@sha256:91ae27d5bb72d80987163b4e673d7f16ad1ef123c5dd6ef141d4417e244e4589, test:rust/crates/urzua-core/src/rules/action.rs#a_variant_a_rule_constructs_is_declared_in_the_table@sha256:c7f3474ae94e6619a10c0a401dda5e43d24901ebf3ebe15836cd68b1bffa850a, test:rust/crates/urzua-core/src/rules/action.rs#an_exemption_states_a_reason_and_an_action_a_valid_op@sha256:2f73475f8f5e6968da67ae22c2cbaae6bbd62253f32ecd4294784d7c26bb308d, spec:docs/specs/SPEC-2-urzua-check.md, code:scripts/generate-rule-table.py, code:.github/workflows/checks.yml
Stable-Id: 01M1YN83N5WHAE63XN53HTDGHR
Phase: '0'
Track: schema-governance
Implements: RFC-8
Blocked-on: —
---
# 44 — Implement suggested_action on every finding

## What

Add `suggested_action` to the `Finding` struct and populate it for at least the rules where a fix is mechanically obvious (e.g. `field.quality`'s blank/placeholder cases), matching the output contract SPEC-2 already documents.

## Why

SPEC-2's own contract and RFC-8 both already describe `suggestedAction` as part of the JSON shape -- checked directly against `report.rs`'s real `Finding` struct, it isn't there. Decided, never built; a finding today says what's wrong but never what to do about it.

## Done means

- `Finding` carries a `suggested_action` in the shape `ADR-73` decides, in the JSON output as `suggested_action`. — evidence: code:rust/crates/urzua-core/src/report.rs, code:rust/crates/urzua-core/src/rules/action.rs, test:rust/crates/urzua-cli/tests/suggested_action_integration.rs#a_finding_carries_the_action_its_variant_declares@sha256:622ae8868e6d433479b304098a5806870a70d74d5388f726f6e0f6609e25ad61
- Every finding variant of every shipped rule declares an action or a reasoned exemption in the rule table, as `ADR-73`'s appendix lists. — evidence: code:rust/crates/urzua-core/src/rules/table.rs, test:rust/crates/urzua-cli/tests/suggested_action_integration.rs#rules_lists_each_variant_with_its_action_or_its_exemption_reason@sha256:91ae27d5bb72d80987163b4e673d7f16ad1ef123c5dd6ef141d4417e244e4589
- A coverage test fails a variant with neither. — evidence: test:rust/crates/urzua-core/src/rules/action.rs#a_variant_a_rule_constructs_is_declared_in_the_table@sha256:c7f3474ae94e6619a10c0a401dda5e43d24901ebf3ebe15836cd68b1bffa850a, test:rust/crates/urzua-core/src/rules/action.rs#an_exemption_states_a_reason_and_an_action_a_valid_op@sha256:2f73475f8f5e6968da67ae22c2cbaae6bbd62253f32ecd4294784d7c26bb308d
- `SPEC-2` reproduces `ADR-73`'s per-kind table and agrees with the rule table. — evidence: spec:docs/specs/SPEC-2-urzua-check.md, code:scripts/generate-rule-table.py
- `urzua check` over urzua's own corpus emits it. — evidence: code:.github/workflows/checks.yml

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Re-phased from Phase 1 to Phase 0 (pulled into Phase 0 as foundation work); `Status` unchanged. **Why:** Maintainer decision of 2026-11-04 defining Phase 0 as parity on a solid codebase (`SPEC-1`'s Phase 0 exit): `SPEC-2` documents `suggestedAction` on a finding and the `Finding` struct lacks it; the structured-output gap is cheapest to close while few consumers exist, and it touches every rule constructor, so it belongs before more rules land. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-04 | Done means rewritten to `ADR-73`'s accepted shape. **Why:** `ADR-73` widens scope from "rules where a fix is mechanically obvious" to every finding variant, with a declared exemption in place of silent omission, and spells the field `suggested_action` as `SPEC-2`'s snake_case does. | **substantive** |
> | 2026-11-04 | `Status` Planned -> Done; `Realized-by` added. **Why:** built as `ADR-73` decides. `Finding.suggested_action` (`report.rs`) carries `kind`, `op`, a flat `target` and a `value` only where derivable, each omitted when not computed; every finding variant of all 53 rules declares an action or a reasoned exemption in `RULE_TABLE` (`rules/action.rs`), a coverage test fails a variant with neither, `urzua rules` lists each variant's action or exemption reason, `SPEC-2` reproduces the per-kind table and agrees with the rule table (a generated action table), and the acceptance expectations assert the shape. A `run-command` is offered only where running it removes the finding (tested for `fix` and `index --apply`). The editor hook is not wired to findings. This repository's own corpus emits no unwaived finding, so the run over it shows only the exempt waived `citation.resolution` ones carrying no action; the action shape is proved on the planted corpora. Variants `ADR-73`'s appendix did not name were declared by the nearest listed rule, as the `ADR-73` and `SPEC-2` tables record. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/suggested_action_integration.rs#rules_lists_each_variant_with_its_action_or_its_exemption_reason` (lines 493-551) 1ec2f6d80ba2 -> 91ae27d5bb72. Reason: Read the diff of the pinned test: rules_lists_each_variant_with_its_action_or_its_exemption_reason changed its expected rule count from 67 to 69 for the two new rules, whose variants carry an action or an exemption reason; the per-variant assertions are unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
