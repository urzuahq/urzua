---
Stable-Id: 01M6HZ0NMJDR3KN60CTNJ4CQ6P
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Realized-by: code:rust/crates/urzua-core/src/config.rs@sha256:df8b21e9c8a5c8a9f829242ac3e16fa71e39a0afaa89ef19a59763cbb2fcfe84, test:rust/crates/urzua-cli/tests/field_levels_integration.rs#a_blank_deciders_is_an_error_and_a_blank_reviewers_a_warning_observed_failing@sha256:f1dd66e516f09275b9a9afa48e1537f891bff7cad340b450ddf0bf1059746fe8, code:rust/crates/urzua-cli/src/gate.rs@sha256:0923b079289476e4348ac7b36e0738d5b4386e68a9684a56a6f7024d0f1ffd24, test:rust/crates/urzua-cli/tests/field_levels_integration.rs#only_a_blank_reviewers_warns_and_does_not_block_observed_failing@sha256:ec03d0d698e9a348f3ac6c54808c6638c2d174280472c2238d4dee5e136b4a00, test:rust/crates/urzua-cli/tests/field_levels_integration.rs#an_unlisted_field_takes_the_rule_level@sha256:f79a7a434ea8031376129eb43d889eb748f5aca528f63d368babbfd6ab303368, test:rust/crates/urzua-cli/tests/field_levels_integration.rs#without_the_option_every_finding_takes_the_rule_level@sha256:6684255a89ee1d3fa8d05808ece7624f222776e0a74e44082f13d8ec0a2002a3, spec:docs/specs/SPEC-3-configuration.md, spec:docs/milestones/MILE-140-per-field-severity-within-one-rule.md
---
# 140 — Per-field severity within one rule

## What

Severity is set per field within one rule, not only per rule. A rule such as `field.quality` can raise a blank value in one field to an error and the same state in another field to a warning, from configuration.

## Why

`MILE-80` made severity configurable per rule, so one rule has one level for every field it covers. The adopter blocks on a blank Deciders on an accepted record but only warns on a blank Reviewers, and those two cannot coexist while `field.quality` has a single level. Splitting the rule into one rule per field is the alternative; it multiplies rule ids for what is one check, so it is worth deciding explicitly which of the two to build.

## Done means

- A config can give two fields of one rule different severities. — evidence: code:rust/crates/urzua-core/src/config.rs@sha256:df8b21e9c8a5c8a9f829242ac3e16fa71e39a0afaa89ef19a59763cbb2fcfe84, test:rust/crates/urzua-cli/tests/field_levels_integration.rs#a_blank_deciders_is_an_error_and_a_blank_reviewers_a_warning_observed_failing@sha256:f1dd66e516f09275b9a9afa48e1537f891bff7cad340b450ddf0bf1059746fe8
- A finding carries the severity of its own field. — evidence: code:rust/crates/urzua-cli/src/gate.rs@sha256:0923b079289476e4348ac7b36e0738d5b4386e68a9684a56a6f7024d0f1ffd24, test:rust/crates/urzua-cli/tests/field_levels_integration.rs#only_a_blank_reviewers_warns_and_does_not_block_observed_failing@sha256:ec03d0d698e9a348f3ac6c54808c6638c2d174280472c2238d4dee5e136b4a00
- An unlisted field takes the rule-level severity. — evidence: test:rust/crates/urzua-cli/tests/field_levels_integration.rs#an_unlisted_field_takes_the_rule_level@sha256:f79a7a434ea8031376129eb43d889eb748f5aca528f63d368babbfd6ab303368, test:rust/crates/urzua-cli/tests/field_levels_integration.rs#without_the_option_every_finding_takes_the_rule_level@sha256:6684255a89ee1d3fa8d05808ece7624f222776e0a74e44082f13d8ec0a2002a3
- The choice between per-field severity and splitting the rule is recorded. — evidence: spec:docs/specs/SPEC-3-configuration.md, spec:docs/milestones/MILE-140-per-field-severity-within-one-rule.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the adopter's linter blocks on one blank role field and warns on another, which urzua's per-rule severity (`MILE-80`) cannot express. | **substantive** |
> | 2026-11-04 | Built the narrow reading: a `field_levels` rule option on `field.quality`, `field.pending` and `status.required-field-missing`, a per-finding level the `check` gate honors, and an unchanged rule-level fallback. **Why:** the chosen alternative to splitting the rule into one rule per field, which would multiply rule ids for one check. Left open: `field.untrimmed-value`, `field.leading-reserved-indicator` and `status.forbidden-field-present` do not take the option, `off` per field is refused, and per-type or per-status levels are not built. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #177: the `field_levels` option gives two fields of one rule different severities. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/config.rs@sha256:18818ee3b226, rust/crates/urzua-cli/src/gate.rs@sha256:0923b0792894 Reason: Pinned the files as they are now after checking each against the record it evidences: status_model.rs and rules/status.rs still declare and judge the per-type state machine (MILE-132), acceptance.rs and the corpus-h gate expectation still hold the held-source case (MILE-133), rules/index.rs still compares generated indexes with the records (MILE-139), config.rs still reads field_levels and gate.rs still derives the report status (MILE-140), refs_snapshot.rs and commands/refs.rs still keep and refresh the cited-number snapshot (MILE-143), the corpus-j expectations still plant an unlisted edge and an unreachable status (MILE-144), config.rs still reads fail_on (MILE-145), claim_field_integration.rs still carries a Verifies back-pointer comment of the kind the milestone's own-corpus item names (MILE-146), main.rs and discovery.rs still read the full back-pointer id list and the decision-gate sources (MILE-147), and embodiment.rs still reads sticky_states (MILE-148). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/config.rs` 18818ee3b226 -> df8b21e9c8a5. Reason: split parse_with_document_models, validate_relation_pairs, claim_expiry_problems and revision_log_problem in config.rs into named validation helpers (SPEC-22 100-line bound); behaviour unchanged, outputs byte-identical, validation order and messages pinned by the new validation_order_tests **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
