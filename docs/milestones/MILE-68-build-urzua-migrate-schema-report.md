---
Status: Done
Stable-Id: 01M1YNNFS3073SZAAP376CD603
Phase: '0'
Track: schema-governance
Implements: ADR-22
Blocked-on: —
Realized-by: test:rust/crates/urzua-core/src/migrate.rs#a_record_missing_the_candidate_field_is_reported_observed_failing@sha256:2b60f82b5d919a7ccac270071a88d30b2e5ed328334c89d62bd691ef6ec33969, test:rust/crates/urzua-cli/tests/template_placeholder_integration.rs#migrate_schema_report_judges_a_field_as_field_quality_does@sha256:ab950e45382fbf4c53e23a879339c0a423e6e83ee19835570b020dec9b7594cf, code:rust/crates/urzua-core/src/migrate.rs@sha256:a8b30583f9658b1ffdcd5920fb2bd08adf9e41bd55d406ac94aaaaf5f59fe768, code:rust/crates/urzua-cli/src/commands/migrate.rs#run_schema_report_inner@sha256:1de46021a1b21618ccce3b7149bafd2ccbbd369b1874064f68b5a27473e22647, test:rust/crates/urzua-cli/tests/could_not_run_integration.rs#every_command_that_reads_a_repository_exits_2_outside_one_observed_failing@sha256:8b3362d317c4fb9b5063b83f918fa80b2d8ee559c7c1e7570e697704f5efcbf5, spec:docs/specs/SPEC-14-urzua-migrate.md
---
# 68 — Build urzua migrate schema --report

## What

`urzua migrate schema --report --field <Name>`: a read-only preview listing every record that would newly fail if `Name` were added to `required_fields` today, before it's ever added to config.

## Why

Adding a required field to a live corpus is a real risk of silently breaking every record that doesn't have it yet -- this answers "what would break" before the config change that would cause it.

## Done means

- `urzua migrate schema --report --field <Name>` lists, read-only, every record that would newly fail if `Name` were added to `required_fields` today. — evidence: test:rust/crates/urzua-core/src/migrate.rs#a_record_missing_the_candidate_field_is_reported_observed_failing@sha256:2b60f82b5d919a7ccac270071a88d30b2e5ed328334c89d62bd691ef6ec33969, test:rust/crates/urzua-cli/tests/template_placeholder_integration.rs#migrate_schema_report_judges_a_field_as_field_quality_does@sha256:ab950e45382fbf4c53e23a879339c0a423e6e83ee19835570b020dec9b7594cf, code:rust/crates/urzua-core/src/migrate.rs@sha256:a8b30583f9658b1ffdcd5920fb2bd08adf9e41bd55d406ac94aaaaf5f59fe768
- `migrate schema --report` writes nothing and changes no config. — evidence: code:rust/crates/urzua-cli/src/commands/migrate.rs#run_schema_report_inner@sha256:1de46021a1b21618ccce3b7149bafd2ccbbd369b1874064f68b5a27473e22647
- The command is exercised through the binary and specified in `SPEC-14`. — evidence: test:rust/crates/urzua-cli/tests/could_not_run_integration.rs#every_command_that_reads_a_repository_exits_2_outside_one_observed_failing@sha256:8b3362d317c4fb9b5063b83f918fa80b2d8ee559c7c1e7570e697704f5efcbf5, spec:docs/specs/SPEC-14-urzua-migrate.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-06 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 1 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/migrate.rs@sha256:58ae3634352f Reason: Pinned the files as they are now after checking each against the record it evidences: report.rs, config.rs and runner.rs still carry the info level (MILE-46), init.rs still proposes a config in adopt mode (MILE-57), doctor.rs, the Makefile, both git hooks and checks.yml still report and wire the gate (MILE-58), waiver.rs, the waiver template and the WAIVER-0035 record still define and hold a reviewed exception (MILE-59), new.rs and new_record.rs still create the next record (MILE-60), both fix files and fix_drift_integration.rs still detect and apply (MILE-61), pointer.rs, audit.rs and the corpus-b expectation still run the audit that reports one-sided supersession (MILE-62), explain.rs and graph.rs still print the governing records and the edges (MILE-63), the corpus-a expectation still shows a config naming the removed bold-list shape rejected, which is what the milestone's item says (MILE-65), and migrate.rs still reports what a newly required field would break without writing (MILE-68). MILE-46's revision table had a blank line between its header and its rows, which hid every row from the parser, so that line was removed before pinning. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/migrate.rs` 58ae3634352f -> a8b30583f965. Reason: Read the diff of each re-pinned file against main. core/src/migrate.rs: Notice struct literals gained detail: None; no behaviour change. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
