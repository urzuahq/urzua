---
Status: Done
Stable-Id: 01M1YNND3FMY2F2A0N3K0FVT3H
Phase: '0'
Track: governance-process
Implements: ADR-24
Blocked-on: —
Realized-by: test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs#explain_names_the_governing_record_and_graph_reports_kind_and_dangling_per_edge@sha256:2dfa7fba6ef52a989d113ad2a97aa02380d6bf8e4324f952ce3704767195d15d, code:rust/crates/urzua-cli/src/commands/explain.rs@sha256:15d3bc36d94402f876239986ac421f37cf23d558db6283c41f2eddbcf09ab71c, code:rust/crates/urzua-cli/src/commands/graph.rs@sha256:379b00875a633e9d8e59f690e2a12ce3ed3b0d5635d533f0a8242580041bfd0a, test:rust/crates/urzua-cli/tests/check_integration.rs#graph_and_explain_report_a_population_and_name_an_unreadable_record_observed_failing@sha256:8d78819250a4b85ba75632a954072ff3d96a3e088b8411123b19ed5bc6726930, spec:docs/specs/SPEC-13-urzua-explain-and-urzua-graph.md
---
# 63 — Build urzua explain and urzua graph

## What

`urzua explain <path>`: every record whose `Realized-by` names that file as evidence. `urzua graph`: the full `Implements`/`Derives-from`/`Supersedes` relationship graph as data, flagging edges that don't resolve.

## Why

The record graph existed in the schema from the start but nothing surfaced it as queryable data -- `explain` answers "which decisions govern this file," the highest-frequency read an agent or reviewer actually has.

## Done means

- `urzua explain <path>` lists every record whose `Realized-by` names that file. — evidence: test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs#explain_names_the_governing_record_and_graph_reports_kind_and_dangling_per_edge@sha256:2dfa7fba6ef52a989d113ad2a97aa02380d6bf8e4324f952ce3704767195d15d, code:rust/crates/urzua-cli/src/commands/explain.rs@sha256:15d3bc36d94402f876239986ac421f37cf23d558db6283c41f2eddbcf09ab71c
- `urzua graph` emits the `Implements`/`Derives-from`/`Supersedes` graph as data with unresolved edges flagged. — evidence: test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs#explain_names_the_governing_record_and_graph_reports_kind_and_dangling_per_edge@sha256:2dfa7fba6ef52a989d113ad2a97aa02380d6bf8e4324f952ce3704767195d15d, code:rust/crates/urzua-cli/src/commands/graph.rs@sha256:379b00875a633e9d8e59f690e2a12ce3ed3b0d5635d533f0a8242580041bfd0a
- Both `explain` and `graph` are read-only. — evidence: code:rust/crates/urzua-cli/src/commands/explain.rs@sha256:15d3bc36d94402f876239986ac421f37cf23d558db6283c41f2eddbcf09ab71c, code:rust/crates/urzua-cli/src/commands/graph.rs@sha256:379b00875a633e9d8e59f690e2a12ce3ed3b0d5635d533f0a8242580041bfd0a
- Both are covered in `check_integration.rs`. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#graph_and_explain_report_a_population_and_name_an_unreadable_record_observed_failing@sha256:8d78819250a4b85ba75632a954072ff3d96a3e088b8411123b19ed5bc6726930
- Both are documented in `SPEC-13`. — evidence: spec:docs/specs/SPEC-13-urzua-explain-and-urzua-graph.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-06 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/commands/explain.rs@sha256:9cdffc6765ee, rust/crates/urzua-cli/src/commands/graph.rs@sha256:379b00875a63 Reason: Pinned the files as they are now after checking each against the record it evidences: report.rs, config.rs and runner.rs still carry the info level (MILE-46), init.rs still proposes a config in adopt mode (MILE-57), doctor.rs, the Makefile, both git hooks and checks.yml still report and wire the gate (MILE-58), waiver.rs, the waiver template and the WAIVER-0035 record still define and hold a reviewed exception (MILE-59), new.rs and new_record.rs still create the next record (MILE-60), both fix files and fix_drift_integration.rs still detect and apply (MILE-61), pointer.rs, audit.rs and the corpus-b expectation still run the audit that reports one-sided supersession (MILE-62), explain.rs and graph.rs still print the governing records and the edges (MILE-63), the corpus-a expectation still shows a config naming the removed bold-list shape rejected, which is what the milestone's item says (MILE-65), and migrate.rs still reports what a newly required field would break without writing (MILE-68). MILE-46's revision table had a blank line between its header and its rows, which hid every row from the parser, so that line was removed before pinning. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/explain.rs` 9cdffc6765ee -> 15d3bc36d944, `rust/crates/urzua-cli/tests/check_integration.rs#graph_and_explain_report_a_population_and_name_an_unreadable_record_observed_failing` (lines 846-893) 1fa8afd36075 -> 8d78819250a4. Reason: the tests in check_integration.rs now name real tracked files; the behaviour this record relies on is unchanged; explain.rs now errors (not-run, exit 2) on a path or glob that matches no tracked file and is cited by no record; explain's behaviour changed only for that case (an unmatched, uncited path or glob); the existing query behaviour this record relies on is not touched by the diff **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
