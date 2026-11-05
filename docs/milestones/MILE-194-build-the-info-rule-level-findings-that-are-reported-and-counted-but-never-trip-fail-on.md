---
Stable-Id: 01M6NH5S2Q1Q7MC6REKNXV9Y7C
Status: Done
Phase: '0'
Track: governance-process
Implements: ADR-87
Blocked-on: null
Realized-by: code:rust/crates/urzua-core/src/config.rs, code:rust/crates/urzua-core/src/report.rs, code:rust/crates/urzua-core/src/rules/runner.rs, test:rust/crates/urzua-cli/tests/info_level_integration.rs, spec:docs/specs/SPEC-2-urzua-check.md, spec:docs/specs/SPEC-3-configuration.md
---
# 194 — Build the info rule level: findings that are reported and counted but never trip fail_on

## What

A `level: info` for rules (and `field_levels`), a `FindingSeverity::Info` serialized `"info"`, and the rule that an info finding is reported and counted but never sets `blocking` and never trips `fail_on`, whatever its value (`ADR-87`). `fail_on: info` is refused at load.

## Why

`ADR-87` (Accepted 2026-11-05) adds the level so advisory rules can be enabled in a repository that fails on warnings. `MILE-189`'s rule reports 308 findings here and cannot be enabled at `warn` without red CI.

## Done means

- `level: info` loads for a rule and for a `field_levels` entry; any other level, and `fail_on: info`, are refused at load with a message listing the valid values; a config without `info` loads and reports byte-identically to before.
- A planted rule at `info` reports its findings with `severity: "info"`, rule, file, line, population and its `ADR-73` suggested action, counts them in `findings` and `status` (`findings-present`), and the run exits 0 under `fail_on: warning` and under `fail_on: error`, with `blocking: false`; the same findings at `warn` exit 1 under `fail_on: warning`. A waiver covers an info finding like any other. These tests fail before the change and pass after.
- `.urzua/config.yaml` enables `embodiment.locator-on-back-pointer-path` at `info`, `urzua check` reports its findings and exits 0 with 0 unwaived warnings or errors, and `MILE-189` records the evidence and moves to Done in the same pull request.
- `SPEC-2` (severity and output contract) and `SPEC-3` (`level`, `field_levels`, `fail_on`) describe the level, with revision rows, and `make rule-table-check` passes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** `ADR-87` (Proposed) decides the `info` level on a maintainer decision of 2026-11-05 and no milestone builds it; it is also what lets `MILE-189`'s Done means be met here. | **substantive** |
> | 2026-11-05 | Built: `level: info` for a rule, a `field_levels` entry and a boundary `severity`; `FindingSeverity::Info` serialized `"info"`; a rule set to `info` caps every declared severity at `info` (point 10); every level-to-severity mapping is exhaustive (`RuleLevel::severity`, `BoundarySeverity::severity`), so `published_paths` at any level no longer falls back to `error`; `fail_on: info` and any other unrecognised level are refused at load. `.urzua/config.yaml` enables `MILE-189`'s rule at `info` and `urzua check` exits 0 with its findings reported. `SPEC-2` and `SPEC-3` describe the level. `Realized-by` added and `Status` set to `Done`. **Why:** every item of Done means is met and exercised in this repository's configuration. | **substantive** |
