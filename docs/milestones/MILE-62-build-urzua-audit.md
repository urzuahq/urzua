---
Status: Done
Stable-Id: 01M1YNNCJQKWVEVQWPQ3ZCHJMV
Phase: '0'
Track: governance-process
Implements: ADR-30
Blocked-on: —
Realized-by: test:rust/crates/urzua-cli/tests/check_integration.rs#audit_honours_the_declared_level_of_a_rule_instead_of_running_it_unconditionally@sha256:60fd2f4ae8378d956ed2d8a41ae62bfb70201e8a496c2d508bf0c8b66956f217, test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#audit_is_check_over_the_configured_rules_plus_one_deprecation_notice@sha256:e190a2681bbc21edee5066f08c4a3ebc6e1d33fb3e089fa68b551839c37b4dc5, code:rust/crates/urzua-core/src/rules/pointer.rs@sha256:2d164d27a2e9d20a589adeee671eaae8db4d5e6529b3e705e9df1d1b0ed6e5ae, code:rust/crates/urzua-cli/src/commands/audit.rs@sha256:7f54effa99c27c74c39869d05b39f69b13b2c030c24d2bd56db72fde59e86817, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-b/expectations/audit-reports-one-sided-supersession.yaml@sha256:a0154b78bc1bc5a9db2c2192d22a908df2b0c6eda6553bb14b8c66446a8fb376, spec:docs/specs/SPEC-11-urzua-audit.md
---
# 62 — Build urzua audit

## What

`urzua audit`: cross-record reconciliation -- supersession reciprocity and dangling cross-references -- reusing `check`'s own rule functions rather than a second implementation. Read-only; never writes.

## Why

A bulk cross-reference rewrite is a real data-loss risk without a review step, so `audit` reports what's wrong across records without ever attempting to fix it itself.

## Done means

- `urzua audit` reports supersession reciprocity across records. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#audit_honours_the_declared_level_of_a_rule_instead_of_running_it_unconditionally@sha256:60fd2f4ae8378d956ed2d8a41ae62bfb70201e8a496c2d508bf0c8b66956f217
- `urzua audit` reports dangling cross-references across records. — evidence: test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#audit_is_check_over_the_configured_rules_plus_one_deprecation_notice@sha256:e190a2681bbc21edee5066f08c4a3ebc6e1d33fb3e089fa68b551839c37b4dc5, code:rust/crates/urzua-core/src/rules/pointer.rs@sha256:2d164d27a2e9d20a589adeee671eaae8db4d5e6529b3e705e9df1d1b0ed6e5ae
- It does so by calling `check`'s own rule functions rather than a second implementation. — evidence: code:rust/crates/urzua-cli/src/commands/audit.rs@sha256:7f54effa99c27c74c39869d05b39f69b13b2c030c24d2bd56db72fde59e86817, test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#audit_is_check_over_the_configured_rules_plus_one_deprecation_notice@sha256:e190a2681bbc21edee5066f08c4a3ebc6e1d33fb3e089fa68b551839c37b4dc5
- `urzua audit` is read-only and writes nothing. — evidence: code:rust/crates/urzua-cli/src/commands/audit.rs@sha256:7f54effa99c27c74c39869d05b39f69b13b2c030c24d2bd56db72fde59e86817
- The acceptance suite runs it (`SPEC-11`). — evidence: test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-b/expectations/audit-reports-one-sided-supersession.yaml@sha256:a0154b78bc1bc5a9db2c2192d22a908df2b0c6eda6553bb14b8c66446a8fb376, spec:docs/specs/SPEC-11-urzua-audit.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-06 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/rules/pointer.rs@sha256:e4d8db46f2e3, rust/crates/urzua-cli/src/commands/audit.rs@sha256:80228f66e936, rust/crates/urzua-cli/tests/acceptance/corpora/corpus-b/expectations/audit-reports-one-sided-supersession.yaml@sha256:a0154b78bc1b Reason: Pinned the files as they are now after checking each against the record it evidences: report.rs, config.rs and runner.rs still carry the info level (MILE-46), init.rs still proposes a config in adopt mode (MILE-57), doctor.rs, the Makefile, both git hooks and checks.yml still report and wire the gate (MILE-58), waiver.rs, the waiver template and the WAIVER-0035 record still define and hold a reviewed exception (MILE-59), new.rs and new_record.rs still create the next record (MILE-60), both fix files and fix_drift_integration.rs still detect and apply (MILE-61), pointer.rs, audit.rs and the corpus-b expectation still run the audit that reports one-sided supersession (MILE-62), explain.rs and graph.rs still print the governing records and the edges (MILE-63), the corpus-a expectation still shows a config naming the removed bold-list shape rejected, which is what the milestone's item says (MILE-65), and migrate.rs still reports what a newly required field would break without writing (MILE-68). MILE-46's revision table had a blank line between its header and its rows, which hid every row from the parser, so that line was removed before pinning. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/audit.rs` 80228f66e936 -> 7f54effa99c2. Reason: Read the diff of each re-pinned file against main. cli/src/commands/audit.rs: Notice struct literals gained detail: None; no behaviour change. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/pointer.rs` e4d8db46f2e3 -> 2d164d27a2e9. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: pointer.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
