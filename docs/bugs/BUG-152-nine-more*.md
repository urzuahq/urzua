---
Stable-Id: 01M6D24G08YR10FW9QP3F28WMW
Status: Fixed
Found-in: "MILE-119, writing an all-rules header_shape: none test"
Realized-by: test:rust/crates/urzua-cli/tests/check_integration.rs#every_header_reading_rule_reports_a_header_none_type_as_out_of_scope@sha256:d78c8597d372fec315366939ff8f986cc1619b38dcd707612d468ab06fd7d66c
---
# 152 — nine more rules misclassify header-shape-none slots as unreadable or absent instead of out of scope

## What was wrong

Nine rules built a per-record or per-slot population with no `header_shape: none` gate (`ADR-50`), so
a record of such a type was counted `Unreadable` (or `Absent`, for `header.layout-consistency`)
instead of `OutOfScope`: `header.layout-consistency`, `pointer.resolution`, `pointer.target-status`,
`narrative-field.stale`, `embodiment.consistency`, `embodiment.locator-exists`,
`embodiment.locator-promotion-candidate`, `relation.supersession-reciprocity` and
`relation.target-status-undeclared`. A type with no header has nowhere for a header to fail to parse,
so the census reported a malformed corpus where the configuration simply declared none.

## Why nothing caught it

`BUG-137`, `BUG-142` and `BUG-147` each swept only the rules a review pass looked at, and gating was
written separately inside each rule. Nothing enumerated every shipped rule against a `none` type.

## Fix

`scoped_census_records` in `rules.rs` applies the classification once; every rule that reads a header
builds its population through it, and the four per-rule gates were removed. The all-rules test iterates
`ALL_RULES` and names each exemption (rules that never read a header). Observed failing on 13 rules
(these nine plus the four `BUG-147` rules) before the change, passing after.

## References

- `MILE-119`; `BUG-98`, `BUG-124`, `BUG-137`, `BUG-142`, `BUG-147`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed and fixed with `MILE-119`'s shared population builder. **Why:** the same `ADR-50` gating gap recurred in nine rules beyond the five already fixed. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#every_header_reading_rule_reports_a_header_none_type_as_out_of_scope` (lines 3342-3455) 1cff6c8d0d7d -> 88bdb8265ec6. Reason: Read the diff of the pinned test: every_header_reading_rule_reports_a_header_none_type_as_out_of_scope gained one config arm giving embodiment.claim-expired the periods and anchor_field it needs to load, and its assertions are unchanged, so the test still requires every non-exempt rule in the table, the two new expiry rules included, to count a header_shape none type as out of scope. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#every_header_reading_rule_reports_a_header_none_type_as_out_of_scope` (lines 3477-3593) 88bdb8265ec6 -> 70b0e1a35141. Reason: Read the diff of each re-pinned file against main and the claim still holds. check_integration.rs#every_header_reading_rule_reports_a_header_none_type_as_out_of_scope: the generated rules config gains an arm declaring back_pointer_paths and extensions for code.dangling-bug-reference; no assertion changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#every_header_reading_rule_reports_a_header_none_type_as_out_of_scope` (lines 3491-3607) 70b0e1a35141 -> 1a3343ae108c. Reason: Adds declared target types for reference fields (MILE-259). config.rs: a pointer_fields entry may be {name, targets} (the typed parse keeps only names), Config gains a skipped pointer_targets map, RelationPair gains optional target_types, a new load error InvalidPointerTargets; every other key unchanged. rules: new rule pointer.target-type (catalog.rs, pointer.rs, table.rs, default-on at error, not-enabled when nothing is declared); relation.rs cross-type-disallowed defers to it for a claim outside target_types. graph.rs lists relation-pair edges. fix.rs, migrate.rs, property.rs, type_scope.rs, sequence.rs: test fixtures add the new fields only. check_integration.rs: the header-none fixture declares a target list so the new rule is exercised. SPEC-3 documents the keys. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#every_header_reading_rule_reports_a_header_none_type_as_out_of_scope` (lines 3491-3607) 1a3343ae108c -> b68b46c9b609. Reason: check_integration.rs: merged main, whose header-none census fixture declares Parent as {name, targets}; this branch adds field_values to the same fixture so field.value-not-declared is held to the out-of-scope census **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-20 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#every_header_reading_rule_reports_a_header_none_type_as_out_of_scope` (lines 3491-3610) b68b46c9b609 -> d78c8597d372. Reason: section.decided-body-changed was added: a RULE_ constant in catalog.rs, a RuleDef entry in table.rs, a module line in rules.rs, DecidedBodyOpts in options.rs, two RuleSetting options and a load check in config.rs, the merge base's copy read for the new rule in check.rs, one id in NOT_YET_RELEASED and a header-none test arm in the integration tests, and SPEC-3 and SPEC-2 prose and rows. Existing rules and every claim each record cites are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
