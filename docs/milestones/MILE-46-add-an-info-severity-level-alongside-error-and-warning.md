---
Status: Done
Stable-Id: 01M1YN84QAGGE2RJG9DFN0MB59
Phase: '1'
Track: schema-governance
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/config.rs, code:rust/crates/urzua-core/src/report.rs, code:rust/crates/urzua-core/src/rules/runner.rs, test:rust/crates/urzua-cli/tests/info_level_integration.rs, code:.urzua/config.yaml
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

`Severity::Info` exists, an info finding is reported but never counts toward `blocking` or `fail_on`, the JSON output and the exit code treat it so, and a test covers each; the relation to per-rule configurable severity (`MILE-80`) is stated, and a rule that is info-shaped today (`embodiment.locator-promotion-candidate`) is declared info in urzua's own `.urzua/config.yaml`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-06 | Built by `MILE-194` (`Severity::Info` as `FindingSeverity::Info`, never blocking and never tripping `fail_on`, with a test for each; `info_level_integration.rs`), and `embodiment.locator-promotion-candidate` is now declared `level: info` with `scope: changed` in `.urzua/config.yaml` (the rule takes the option, `ADR-90`, so it nudges only when a record citing a shared locator is touched). `Realized-by` added and `Status` set to `Done`. **Why:** every clause of Done means is met; maintainer decision of 2026-11-06 to enable the rule at `info` here, which closes the one clause the earlier build left open (the config declared it `off`). | **substantive** |
