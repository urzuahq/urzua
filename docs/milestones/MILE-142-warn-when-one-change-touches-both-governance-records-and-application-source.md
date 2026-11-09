---
Stable-Id: 01M6HZ0PHX2W3QNCF6PKCWYNSA
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Realized-by: code:rust/crates/urzua-core/src/rules/diff_gate.rs#source_and_records_mixed@sha256:12b6a156f4c5396f571a1061965ca194227c4a159a6412f08f9213a22c2ce979, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#mixed_change_warns_once_naming_a_path_from_each_side@sha256:7f5744a8ce071c9b15dd30d5ff62b2a74d86f21f0fc4027f7378c5c6f53ee67d, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#a_change_touching_only_one_side_does_not_warn@sha256:515eb5c36bd1f6b6cf241ac69cdca1dec1aeddab4bb859a287f6365982613d03, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#mixed_rule_without_base_is_not_applicable@sha256:0e0562bd3dea530f4ca837e0f4e0e70ce25f9df9590e220e6f978a7f47d5a8da, test:rust/crates/urzua-core/src/rules/diff_gate.rs#no_diff_is_not_applicable_for_both_rules@sha256:944ba0f4d8b972845d55a5be31db7b947922d8951be9aba48ea7d865230d0d3f, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#excluded_paths_are_not_application_source@sha256:14f799805c28ae18105ef41f56e9006f910554784de5fbe2549033b1f9ab2f9c, test:rust/crates/urzua-core/src/rules/diff_gate.rs#exclude_globs_also_narrow_the_mixed_rule@sha256:4d464a1bf9154a474d24e06a6ed31b8f79d1fddea86523fad9d3aa85befca646
---
# 142 — Warn when one change touches both governance records and application source

## What

A warning when a single change touches both governance records and application source. The changed paths are classified by two declared path sets (record directories, application source) and a diff that hits both warns. It runs only where a base ref is available and fails loudly, rather than passing silently, when the requested diff cannot be computed.

## Why

The adopter keeps a warn-only check that flags a pull request mixing a record change with source change, as a prompt to split it. It is the softest of its diff checks. It depends on a diff scope (`MILE-123`) and is the inverse of `MILE-124`, which requires a record change alongside a source change.

## Done means

- A diff touching both path sets produces a warning naming one path from each. — evidence: code:rust/crates/urzua-core/src/rules/diff_gate.rs#source_and_records_mixed@sha256:12b6a156f4c5396f571a1061965ca194227c4a159a6412f08f9213a22c2ce979, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#mixed_change_warns_once_naming_a_path_from_each_side@sha256:7f5744a8ce071c9b15dd30d5ff62b2a74d86f21f0fc4027f7378c5c6f53ee67d
- A diff touching only one set produces no warning. — evidence: test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#a_change_touching_only_one_side_does_not_warn@sha256:515eb5c36bd1f6b6cf241ac69cdca1dec1aeddab4bb859a287f6365982613d03
- With no base ref the rule reports that it did not run rather than reporting clean. — evidence: test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#mixed_rule_without_base_is_not_applicable@sha256:0e0562bd3dea530f4ca837e0f4e0e70ce25f9df9590e220e6f978a7f47d5a8da, test:rust/crates/urzua-core/src/rules/diff_gate.rs#no_diff_is_not_applicable_for_both_rules@sha256:944ba0f4d8b972845d55a5be31db7b947922d8951be9aba48ea7d865230d0d3f
- Test files are excluded from the application set as the config declares. — evidence: test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#excluded_paths_are_not_application_source@sha256:14f799805c28ae18105ef41f56e9006f910554784de5fbe2549033b1f9ab2f9c, test:rust/crates/urzua-core/src/rules/diff_gate.rs#exclude_globs_also_narrow_the_mixed_rule@sha256:4d464a1bf9154a474d24e06a6ed31b8f79d1fddea86523fad9d3aa85befca646

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the adopter's linter warns when one diff mixes governance records with application source, and no urzua rule covers the mixed case. | **substantive** |
> | 2026-11-04 | Built as `diff.source-and-records-mixed` (`rules/diff_gate.rs`): with the declared `application_paths` and optional `exclude_paths` options, one finding naming the first changed record and the first changed application source path when a `--base` diff touches both; the record set is the configured record type directories, and a governed record is never application source. Reports `not-applicable` on a run with no `--base`. `Status` unchanged. **Why:** the diff-scoped run it depends on (`MILE-123`) has merged, so the check could be built. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #169: opt-in rule `diff.source-and-records-mixed`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/diff_gate.rs#source_and_records_mixed` (lines 241-286) 094687ee076d -> 12b6a156f4c5. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: diff_gate.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged; the private not_applicable is removed in favour of the shared helper in shared.rs. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
