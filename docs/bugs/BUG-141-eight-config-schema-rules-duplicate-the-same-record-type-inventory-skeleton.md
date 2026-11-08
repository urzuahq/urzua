---
Stable-Id: 01M373DZ7A16RH97KE4VCPHT66
Status: Fixed
Found-in: "MILE-112's duplication sweep"
Realized-by: test:rust/crates/urzua-core/src/rules/declaration.rs#a_type_with_no_known_fields_declared_is_a_missing_declaration_observed_failing@sha256:377acaa022cf93d5c34474568f74c2a750978863a6380c414dbda2dafb69dde6, test:rust/crates/urzua-core/src/rules/declaration.rs#a_field_present_in_both_pointer_and_narrative_lists_is_an_overlap_observed_failing@sha256:ef4528aac3ad033cfa4bff01e0c9e4a21c09086b94e4bc9e222c88f4b730c51e, test:rust/crates/urzua-core/src/rules/companion.rs#a_type_declaring_one_half_of_a_declared_pair_is_reported_observed_failing@sha256:8c8d2528633801b49d66113f022d8840268ad34c694a3ffdfaf9ca567b8abf84, test:rust/crates/urzua-core/src/rules/type_scope.rs#a_type_with_no_declared_spec_is_a_finding_observed_failing@sha256:56c2ee1aea6d068621ea98413c4532a1ac4f2faf55bba85b07752a0d23aa911f
---
# 141 — eight config-schema rules duplicate the same record-type inventory skeleton

## What was wrong

Eight rules that examine `Config::record_types` itself rather than any record (`type_no_declared_spec`,
`header_deprecated_shape`, `config_pointer_declaration_missing`,
`config_known_fields_declaration_missing`, `config_pointer_field_not_known`,
`config_relation_field_not_known`, `config_pointer_narrative_overlap`,
`config_header_none_has_no_required_fields`) all shared the identical skeleton, differing only in the
rule id, the per-type check, and the message:

```rust
let type_names = config.sorted_type_names();
let population = census(PopulationUnit::RecordType, type_names, |type_name| {
    let type_config = &config.record_types[*type_name];
    /* differs: the check, pushing Finding(s) */
    Outcome::Examined
});
(RuleExecution { rule: RULE_ID.to_string(), population: Some(population), status: RuleStatus::Ran, examined_records: Vec::new() }, findings)
```

Six of these eight functions' own doc comments already say "the same shape as `X`" for another one in
this same list -- the duplication was self-acknowledged in six places and never extracted. Broader than
the already-tracked `RuleExecution`-construction-tail pattern: this is the whole loop body, not only its
tail.

## Why nothing caught it

Each rule was filed, reviewed, and landed independently across several rounds as its own config
validation, so no single review compared its shape against the seven siblings that came before or
after it.

## Fix

Extracted `schema_inventory_rule(config, rule_id, check: impl FnMut(&str, &RecordTypeConfig, &mut
Vec<Finding>)) -> (RuleExecution, Vec<Finding>)`; each of the 8 rules is now just its own check closure,
with the census/`RuleExecution` construction shared. `type_dir_matches_nothing` was not folded in --
it needs `matched`/`dir_exists`, data this shape has no room for, and forcing it in would either widen
the shared signature for one caller or leave it half-fitting.

No behavior change: the full test suite passes unchanged, and the real corpus reports the same 70
findings.

## References

- `BUG-139`/`BUG-140` -- the other two duplication instances the same sweep found and fixed.
- `SPEC-22` -- the duplication-on-second-occurrence rule this was eight occurrences past.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass, found by MILE-112's duplication sweep. **Why:** eight config-schema rules shared one skeleton, self-acknowledged as "the same shape" in six of their own doc comments but never extracted. No behavior change: full suite and real-corpus finding count unchanged. | **substantive** |
> | 2026-11-05 | `Regression-test` retired (ADR-84); the 4 existing tests that cover the change are now cited as `test:` locators in `Realized-by`, and the prose text is removed. **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/type_scope.rs#a_type_with_no_declared_spec_is_a_finding_observed_failing` (lines 340-372) 3d8105ac192c -> 56c2ee1aea6d. Reason: Read the diff of each pinned span since the old pin and the claim still holds. type_scope.rs#a_type_with_no_declared_spec_is_a_finding_observed_failing: the test's Config or report fixture literal gained newly added default fields (for example roadmap, attestation, new, discovery, harness, authority, permanent, rules_selected, include_untracked), assertions unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
