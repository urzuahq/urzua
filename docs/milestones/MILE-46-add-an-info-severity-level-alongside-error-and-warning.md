---
Status: Planned
Stable-Id: 01M1YN84QAGGE2RJG9DFN0MB59
Phase: '1'
Track: schema-governance
Blocked-on: —
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