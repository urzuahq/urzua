---
Stable-Id: 01M6CXZX537SRJ6YWNB7EGAM86
Status: Done
Phase: '0'
Track: schema-governance
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/diff_gate.rs#source_requires_record@sha256:69b833418f1fd6cc52f54f6e3ce5f364bd856f5dac808febc1cad58345bbc105, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#source_change_with_no_record_change_is_an_error_naming_the_path@sha256:0a96e3f9263a3e48a3178687a067c8bcb809b8cf071cf1f65c53449c59d06a82, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#both_rules_need_their_path_options_to_load@sha256:4e82223c0276aabe86422b1137dd4048c0b9fcd16c281dbd0bdcdfd3dc619299, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#a_file_below_a_record_directory_is_not_a_governed_record_change@sha256:1df5979551659dcd30330cab7b6204c478ac596ef61e3077517b541a147ddb72, test:rust/crates/urzua-core/src/rules/diff_gate.rs#the_record_dir_is_whatever_the_config_declares@sha256:61712099dab31c992929a2015a6afd241752addffe5a52d1d73eb3c9ddf7fb59, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#source_change_alongside_a_record_change_is_clean@sha256:6aa98ec9ee800a49d906c4fcb1f82447d2fdce8833e9b88247805dfb5642d695, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#without_base_the_rule_reports_not_applicable_and_the_run_is_not_ok@sha256:f9da18ee4ee59da56e123a03700a14d762a5178cb44283f4b5d225cecc8595a8
---
# 124 — A rule requiring that a diff changing declared source paths also changes a governing record

## What

A rule, declared in config, requiring that a diff which changes declared source paths also changes a
governing record. The paths and the record set are declared; the rule is evaluated against a diff and
reports a source change with no accompanying record change. It depends on a diff-scoped run
(`MILE-123`).

## Why

Replacing a hand-written linter outright (`MILE-102`) means every convention such a linter enforces
needs a declared home, and source-change-requires-record-change is a common one. It is the mechanical half of the
decision-before-implementation idea `MILE-14` leaves undecided: this rule asks only that a record
changed, never that it is the right one, per `AGENTS.md`'s limit on what `check` may claim. `MILE-14`
remains the place to decide whether such a gate should exist by default.

## Done means

- The opt-in rule `diff.source-requires-record` takes declared `source_paths` and, in a `--base` diff, reports one error per changed source path when no governed record changed in the same diff. — evidence: code:rust/crates/urzua-core/src/rules/diff_gate.rs#source_requires_record@sha256:69b833418f1fd6cc52f54f6e3ce5f364bd856f5dac808febc1cad58345bbc105, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#source_change_with_no_record_change_is_an_error_naming_the_path@sha256:0a96e3f9263a3e48a3178687a067c8bcb809b8cf071cf1f65c53449c59d06a82, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#both_rules_need_their_path_options_to_load@sha256:4e82223c0276aabe86422b1137dd4048c0b9fcd16c281dbd0bdcdfd3dc619299
- A governed record is a changed `.md` file directly in a configured record type's `dir`. — evidence: test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#a_file_below_a_record_directory_is_not_a_governed_record_change@sha256:1df5979551659dcd30330cab7b6204c478ac596ef61e3077517b541a147ddb72, test:rust/crates/urzua-core/src/rules/diff_gate.rs#the_record_dir_is_whatever_the_config_declares@sha256:61712099dab31c992929a2015a6afd241752addffe5a52d1d73eb3c9ddf7fb59
- A fixture covers a source change with and without a record change. — evidence: test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#source_change_with_no_record_change_is_an_error_naming_the_path@sha256:0a96e3f9263a3e48a3178687a067c8bcb809b8cf071cf1f65c53449c59d06a82, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#source_change_alongside_a_record_change_is_clean@sha256:6aa98ec9ee800a49d906c4fcb1f82447d2fdce8833e9b88247805dfb5642d695
- The rule depends on the diff-scoped run of `MILE-123`, so without a base it reports that it did not run. — evidence: test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#without_base_the_rule_reports_not_applicable_and_the_run_is_not_ok@sha256:f9da18ee4ee59da56e123a03700a14d762a5178cb44283f4b5d225cecc8595a8

## References

- `MILE-14` -- decide RFC-12's decision-before-implementation gate.
- `MILE-123` -- the diff-scoped run.
- `MILE-102` -- the adopter replacement this serves.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** Replacing a linter outright needs source-change-requires-record-change to have a declared home; filed for that. | **substantive** |
> | 2026-11-04 | Built as `diff.source-requires-record` (`rules/diff_gate.rs`): with the declared `source_paths` option, one error per changed source path when no governed record changed in the same `--base` diff. "A governed record" is the narrowest reading: a changed `.md` file directly in a configured record type's `dir`, deleted ones included; whether it is the right record is not judged. On a run with no `--base` it reports `not-applicable`, never clean. `Status` unchanged. **Why:** filed work is the adopter-replacement rule `MILE-102` needs; the diff-scoped run it waited on (`MILE-123`) has merged. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #169: opt-in rule `diff.source-requires-record`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | `Blocked-on` cleared. **Why:** `MILE-123` (the diff-scoped run) is `Done` and this milestone is `Done`; `narrative-field.stale` flagged the leftover. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/diff_gate.rs#source_requires_record` (lines 131-195) 8c1bf49999c4 -> 69b833418f1f. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: diff_gate.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged; the private not_applicable is removed in favour of the shared helper in shared.rs. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
