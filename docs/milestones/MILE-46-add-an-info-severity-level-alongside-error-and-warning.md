---
Status: Done
Stable-Id: 01M1YN84QAGGE2RJG9DFN0MB59
Phase: '1'
Track: schema-governance
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/report.rs@sha256:5b713d23f7fa371a7ce84c8baab4b6c4d8859177ea12c9a8f18d245a9b860f8b, code:rust/crates/urzua-core/src/config.rs@sha256:18818ee3b2262e4f5dd6b5960d7f874a5662afaabf772f15c82441d7cc7c2df0, code:rust/crates/urzua-core/src/rules/runner.rs@sha256:725f608ea188a00348bf4f4396aca38a87679dd6e21b9fe77832fcc97dee0b64, test:rust/crates/urzua-cli/tests/info_level_integration.rs#an_info_finding_is_reported_counted_and_never_trips_fail_on@sha256:9024e5b06473ecd4159fe00d2e4eaf96e482d1e16d5fb2192e3c4db85a8f7af2, test:rust/crates/urzua-cli/tests/info_level_integration.rs#the_same_finding_at_warn_exits_1_under_fail_on_warning_and_differs_only_in_severity@sha256:b5bf8ded4d558bb6f04016ac3cb245e9cea4c81771c6dbe6b962744f8969032e, spec:docs/specs/SPEC-3-configuration.md, code:.urzua/config.yaml#rules::embodiment.locator-promotion-candidate@sha256:f03e6caf54e4c133f4c70a42e5486808294a3abf805c3f96f44ad90a8bf8c2c1
---
# 46 — Add an info severity level alongside error and warning

## What

Add `Severity::Info` alongside `Error`/`Warning` in `report.rs`, for findings worth surfacing that shouldn't count toward `blocking` or read as a warning.

## Why

Every finding today is Error or Warning -- there's no severity for genuinely informational output (e.g. a future `suggested_action`-only note, or `embodiment.locator-promotion-candidate`'s own "consider promoting" nudge, which is arguably info-shaped today and only Warning because nothing else exists).

This owns *which severity levels exist* in `Severity` itself; MILE-80 owns *making a rule's severity
configurable per repo* (SPEC-1's own unbuilt "which checks are errors vs. warnings" promise) --
related, found independently, kept separate since one is a schema/enum question and the other is a
config-surface question. Resolve which comes first when either is picked up.

## Done means

- `Severity::Info` exists (as `FindingSeverity::Info`). — evidence: code:rust/crates/urzua-core/src/report.rs@sha256:5b713d23f7fa371a7ce84c8baab4b6c4d8859177ea12c9a8f18d245a9b860f8b, code:rust/crates/urzua-core/src/config.rs@sha256:18818ee3b2262e4f5dd6b5960d7f874a5662afaabf772f15c82441d7cc7c2df0
- An info finding is reported but never counts toward `blocking` or `fail_on`, and the JSON output and the exit code treat it so, with a test covering each. — evidence: code:rust/crates/urzua-core/src/rules/runner.rs@sha256:725f608ea188a00348bf4f4396aca38a87679dd6e21b9fe77832fcc97dee0b64, test:rust/crates/urzua-cli/tests/info_level_integration.rs#an_info_finding_is_reported_counted_and_never_trips_fail_on@sha256:9024e5b06473ecd4159fe00d2e4eaf96e482d1e16d5fb2192e3c4db85a8f7af2, test:rust/crates/urzua-cli/tests/info_level_integration.rs#the_same_finding_at_warn_exits_1_under_fail_on_warning_and_differs_only_in_severity@sha256:b5bf8ded4d558bb6f04016ac3cb245e9cea4c81771c6dbe6b962744f8969032e
- The relation to per-rule configurable severity (`MILE-80`) is stated. — evidence: spec:docs/specs/SPEC-3-configuration.md
- A rule that is info-shaped today (`embodiment.locator-promotion-candidate`) is declared info in urzua's own `.urzua/config.yaml`. — evidence: code:.urzua/config.yaml#rules::embodiment.locator-promotion-candidate@sha256:f03e6caf54e4c133f4c70a42e5486808294a3abf805c3f96f44ad90a8bf8c2c1

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-06 | Built by `MILE-194` (`Severity::Info` as `FindingSeverity::Info`, never blocking and never tripping `fail_on`, with a test for each; `info_level_integration.rs`), and `embodiment.locator-promotion-candidate` is now declared `level: info` with `scope: changed` in `.urzua/config.yaml` (the rule takes the option, `ADR-90`, so it nudges only when a record citing a shared locator is touched). `Realized-by` added and `Status` set to `Done`. **Why:** every clause of Done means is met; maintainer decision of 2026-11-06 to enable the rule at `info` here, which closes the one clause the earlier build left open (the config declared it `off`). | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/report.rs@sha256:60b6889bf4eb, rust/crates/urzua-core/src/config.rs@sha256:18818ee3b226, rust/crates/urzua-core/src/rules/runner.rs@sha256:725f608ea188 Reason: Pinned the files as they are now after checking each against the record it evidences: report.rs, config.rs and runner.rs still carry the info level (MILE-46), init.rs still proposes a config in adopt mode (MILE-57), doctor.rs, the Makefile, both git hooks and checks.yml still report and wire the gate (MILE-58), waiver.rs, the waiver template and the WAIVER-0035 record still define and hold a reviewed exception (MILE-59), new.rs and new_record.rs still create the next record (MILE-60), both fix files and fix_drift_integration.rs still detect and apply (MILE-61), pointer.rs, audit.rs and the corpus-b expectation still run the audit that reports one-sided supersession (MILE-62), explain.rs and graph.rs still print the governing records and the edges (MILE-63), the corpus-a expectation still shows a config naming the removed bold-list shape rejected, which is what the milestone's item says (MILE-65), and migrate.rs still reports what a newly required field would break without writing (MILE-68). MILE-46's revision table had a blank line between its header and its rows, which hid every row from the parser, so that line was removed before pinning. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` 60b6889bf4eb -> 5b713d23f7fa. Reason: Read the diff of each re-pinned file against main. core/report.rs: Notice gained an optional detail map, omitted from output when absent; no other reporting changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
