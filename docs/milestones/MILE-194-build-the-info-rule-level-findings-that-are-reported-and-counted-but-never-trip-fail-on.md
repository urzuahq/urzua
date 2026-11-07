---
Stable-Id: 01M6NH5S2Q1Q7MC6REKNXV9Y7C
Status: InProgress
Phase: '0'
Track: governance-process
Implements: ADR-87
Blocked-on: null
Realized-by: test:rust/crates/urzua-cli/tests/info_level_integration.rs#field_levels_accepts_info_and_the_other_field_keeps_its_level@sha256:ce859852409d5a23da1346a326b1c27b9396753d93d10e8f5c3e2013f14b45d9, test:rust/crates/urzua-cli/tests/info_level_integration.rs#an_info_finding_is_reported_counted_and_never_trips_fail_on@sha256:9024e5b06473ecd4159fe00d2e4eaf96e482d1e16d5fb2192e3c4db85a8f7af2, test:rust/crates/urzua-cli/tests/info_level_integration.rs#any_other_level_and_fail_on_info_are_refused_at_load@sha256:5f41e78b418af8234997cd62a009600c583ed85cb1651200d6478236cfd08b01, code:rust/crates/urzua-core/src/config.rs, test:rust/crates/urzua-cli/tests/info_level_integration.rs#the_same_finding_at_warn_exits_1_under_fail_on_warning_and_differs_only_in_severity@sha256:b5bf8ded4d558bb6f04016ac3cb245e9cea4c81771c6dbe6b962744f8969032e, test:rust/crates/urzua-cli/tests/info_level_integration.rs#a_waiver_covers_an_info_finding_like_any_other@sha256:0c541ea80510d2a9e9ea4e84eb47570bd07018ce7a488e5a9eb433b9eb7cdb36, code:.urzua/config.yaml, code:Makefile, code:.github/workflows/checks.yml, code:docs/milestones/MILE-189-an-opt-in-warning-flags-a-realized-by-locator-on-a-file-under-back-pointer-paths-and-nudges-toward-an-implements-or-verifies-comment.md, spec:docs/specs/SPEC-2-urzua-check.md, spec:docs/specs/SPEC-3-configuration.md, code:scripts/generate-rule-table.py
---
# 194 — Build the info rule level: findings that are reported and counted but never trip fail_on

## What

A `level: info` for rules (and `field_levels`), a `FindingSeverity::Info` serialized `"info"`, and the rule that an info finding is reported and counted but never sets `blocking` and never trips `fail_on`, whatever its value (`ADR-87`). `fail_on: info` is refused at load.

## Why

`ADR-87` (Accepted 2026-11-05) adds the level so advisory rules can be enabled in a repository that fails on warnings. `MILE-189`'s rule reports 308 findings here and cannot be enabled at `warn` without red CI.

## Done means

- `level: info` loads for a rule and for a `field_levels` entry. — evidence: test:rust/crates/urzua-cli/tests/info_level_integration.rs#field_levels_accepts_info_and_the_other_field_keeps_its_level@sha256:ce859852409d5a23da1346a326b1c27b9396753d93d10e8f5c3e2013f14b45d9, test:rust/crates/urzua-cli/tests/info_level_integration.rs#an_info_finding_is_reported_counted_and_never_trips_fail_on@sha256:9024e5b06473ecd4159fe00d2e4eaf96e482d1e16d5fb2192e3c4db85a8f7af2
- Any other level, and `fail_on: info`, are refused at load with a message listing the valid values. — evidence: test:rust/crates/urzua-cli/tests/info_level_integration.rs#any_other_level_and_fail_on_info_are_refused_at_load@sha256:5f41e78b418af8234997cd62a009600c583ed85cb1651200d6478236cfd08b01, code:rust/crates/urzua-core/src/config.rs
- A config without `info` loads and reports byte-identically to before.
- A planted rule at `info` reports its findings with `severity: "info"`, rule, file, line, population and its `ADR-73` suggested action, and counts them in `findings` and `status` (`findings-present`). — evidence: test:rust/crates/urzua-cli/tests/info_level_integration.rs#an_info_finding_is_reported_counted_and_never_trips_fail_on@sha256:9024e5b06473ecd4159fe00d2e4eaf96e482d1e16d5fb2192e3c4db85a8f7af2, test:rust/crates/urzua-cli/tests/info_level_integration.rs#the_same_finding_at_warn_exits_1_under_fail_on_warning_and_differs_only_in_severity@sha256:b5bf8ded4d558bb6f04016ac3cb245e9cea4c81771c6dbe6b962744f8969032e
- The run with info findings exits 0 under `fail_on: warning` and under `fail_on: error`, with `blocking: false`; the same findings at `warn` exit 1 under `fail_on: warning`. — evidence: test:rust/crates/urzua-cli/tests/info_level_integration.rs#an_info_finding_is_reported_counted_and_never_trips_fail_on@sha256:9024e5b06473ecd4159fe00d2e4eaf96e482d1e16d5fb2192e3c4db85a8f7af2, test:rust/crates/urzua-cli/tests/info_level_integration.rs#the_same_finding_at_warn_exits_1_under_fail_on_warning_and_differs_only_in_severity@sha256:b5bf8ded4d558bb6f04016ac3cb245e9cea4c81771c6dbe6b962744f8969032e
- A waiver covers an info finding like any other. — evidence: test:rust/crates/urzua-cli/tests/info_level_integration.rs#a_waiver_covers_an_info_finding_like_any_other@sha256:0c541ea80510d2a9e9ea4e84eb47570bd07018ce7a488e5a9eb433b9eb7cdb36
- `.urzua/config.yaml` enables `embodiment.locator-on-back-pointer-path` at `info`. — evidence: code:.urzua/config.yaml
- `urzua check` over this repository reports the rule's findings and exits 0 with 0 unwaived warnings or errors. — evidence: code:Makefile, code:.github/workflows/checks.yml
- `MILE-189` records the evidence and moves to Done in the same pull request. — evidence: code:docs/milestones/MILE-189-an-opt-in-warning-flags-a-realized-by-locator-on-a-file-under-back-pointer-paths-and-nudges-toward-an-implements-or-verifies-comment.md
- `SPEC-2` (severity and output contract) and `SPEC-3` (`level`, `field_levels`, `fail_on`) describe the level, with revision rows. — evidence: spec:docs/specs/SPEC-2-urzua-check.md, spec:docs/specs/SPEC-3-configuration.md
- `make rule-table-check` passes. — evidence: code:scripts/generate-rule-table.py, spec:docs/specs/SPEC-2-urzua-check.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** `ADR-87` (Proposed) decides the `info` level on a maintainer decision of 2026-11-05 and no milestone builds it; it is also what lets `MILE-189`'s Done means be met here. | **substantive** |
> | 2026-11-05 | Built: `level: info` for a rule, a `field_levels` entry and a boundary `severity`; `FindingSeverity::Info` serialized `"info"`; a rule set to `info` caps every declared severity at `info` (point 10); every level-to-severity mapping is exhaustive (`RuleLevel::severity`, `BoundarySeverity::severity`), so `published_paths` at any level no longer falls back to `error`; `fail_on: info` and any other unrecognised level are refused at load. `.urzua/config.yaml` enables `MILE-189`'s rule at `info` and `urzua check` exits 0 with its findings reported. `SPEC-2` and `SPEC-3` describe the level. `Realized-by` added and `Status` set to `Done`. **Why:** every item of Done means is met and exercised in this repository's configuration. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-07 | `Status` `Done` -> `InProgress`. Status set from its Done-means items (ADR-96, MILE-215): unresolved item 3. **Why:** the item has no evidence clause, and `DoneWithUnresolvedItems` forbids `Done` while an item is unresolved; `fix` never downgrades, so the status is set by hand to the computed value. | **structural** |
