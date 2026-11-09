---
Stable-Id: 01M6JW42KJAWDCS570PJFFZM61
Status: Fixed
Found-in: 'a re-run of urzua over a real adopter corpus (2026-11-04)'
Realized-by: test:rust/crates/urzua-core/src/rules/section.rs#a_decisionless_status_is_exempt_and_counted_eligible_and_unexamined_observed_failing@sha256:c52100380e2724e0c97135db6f0130063f177517fe72adbef26c00585f1035cd, test:rust/crates/urzua-cli/tests/section_rules_integration.rs#an_exempted_record_does_not_trigger_scope_matches_nothing_observed_failing@sha256:4f0d16f9dea29af5888fd3542a0acda8cc064e2301c800cb29b19d63bbb70d3b
---
# 187 — Records exempted by exempt_statuses still trigger config.scope-matches-nothing

## What was wrong

A rule's `exempt_statuses` setting sets a record aside by design, yet `config.scope-matches-nothing` counts the exempted records as candidates the configuration does not reach and warns that the rule was handed candidates it cannot examine. The warning is false: the exemption is a declared input, not a misconfiguration. Under `fail_on: warning` it fails the build and the only remedy is a standing waiver. This is the same class as `BUG-186`.

## Why nothing caught it

The `exempt_statuses` tests asserted that an exempt record is skipped; none ran `config.scope-matches-nothing` over the rule's execution, so the outcome chosen for an exempted record (unreachable rather than eligible and unexamined) was never compared with how the meta-rule reads it. `BUG-186` and `MILE-158` fixed the same trap for external ids and diff exclusions, each reviewed one rule at a time.

## References

Same classification as `BUG-186`: an exempted record is eligible and unexamined (set aside on purpose), not out of scope. Done when the change is exercised by an `exempt_statuses` declaration in urzua's own `.urzua/config.yaml` and corpus with no waiver, and `urzua check` over this repository reports it without the warning.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this defect. | **substantive** |
> | 2026-11-04 | Fixed. A record exempted by `sections.exempt_statuses` is counted eligible and unexamined. Exercised in this repository: the ADR type declares `sections` with `exempt_statuses: [Rejected]` and `section.required` is enabled, with no waiver. **Why:** `SPEC-2` 0.45, `SPEC-3` 0.33. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 2 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/section_rules_integration.rs#an_exempted_record_does_not_trigger_scope_matches_nothing_observed_failing` (lines 167-183) 2c9e39acc7f1 -> 4f0d16f9dea2. Reason: Read the diff of each pinned span since the old pin and the claim still holds. section_rules_integration.rs#an_exempted_record_does_not_trigger_scope_matches_nothing_observed_failing: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
