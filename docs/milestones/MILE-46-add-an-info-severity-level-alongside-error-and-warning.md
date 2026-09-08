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

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
