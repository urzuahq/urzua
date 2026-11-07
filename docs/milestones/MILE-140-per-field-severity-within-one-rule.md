---
Stable-Id: 01M6HZ0NMJDR3KN60CTNJ4CQ6P
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Realized-by: code:rust/crates/urzua-core/src/config.rs, test:rust/crates/urzua-cli/tests/field_levels_integration.rs#a_blank_deciders_is_an_error_and_a_blank_reviewers_a_warning_observed_failing@sha256:f1dd66e516f09275b9a9afa48e1537f891bff7cad340b450ddf0bf1059746fe8, code:rust/crates/urzua-cli/src/gate.rs, test:rust/crates/urzua-cli/tests/field_levels_integration.rs#only_a_blank_reviewers_warns_and_does_not_block_observed_failing@sha256:ec03d0d698e9a348f3ac6c54808c6638c2d174280472c2238d4dee5e136b4a00, test:rust/crates/urzua-cli/tests/field_levels_integration.rs#an_unlisted_field_takes_the_rule_level@sha256:f79a7a434ea8031376129eb43d889eb748f5aca528f63d368babbfd6ab303368, test:rust/crates/urzua-cli/tests/field_levels_integration.rs#without_the_option_every_finding_takes_the_rule_level@sha256:6684255a89ee1d3fa8d05808ece7624f222776e0a74e44082f13d8ec0a2002a3, spec:docs/specs/SPEC-3-configuration.md, spec:docs/milestones/MILE-140-per-field-severity-within-one-rule.md
---
# 140 — Per-field severity within one rule

## What

Severity is set per field within one rule, not only per rule. A rule such as `field.quality` can raise a blank value in one field to an error and the same state in another field to a warning, from configuration.

## Why

`MILE-80` made severity configurable per rule, so one rule has one level for every field it covers. The adopter blocks on a blank Deciders on an accepted record but only warns on a blank Reviewers, and those two cannot coexist while `field.quality` has a single level. Splitting the rule into one rule per field is the alternative; it multiplies rule ids for what is one check, so it is worth deciding explicitly which of the two to build.

## Done means

- A config can give two fields of one rule different severities. — evidence: code:rust/crates/urzua-core/src/config.rs, test:rust/crates/urzua-cli/tests/field_levels_integration.rs#a_blank_deciders_is_an_error_and_a_blank_reviewers_a_warning_observed_failing@sha256:f1dd66e516f09275b9a9afa48e1537f891bff7cad340b450ddf0bf1059746fe8
- A finding carries the severity of its own field. — evidence: code:rust/crates/urzua-cli/src/gate.rs, test:rust/crates/urzua-cli/tests/field_levels_integration.rs#only_a_blank_reviewers_warns_and_does_not_block_observed_failing@sha256:ec03d0d698e9a348f3ac6c54808c6638c2d174280472c2238d4dee5e136b4a00
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
