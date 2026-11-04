---
Status: Planned
Stable-Id: 01M1YN86SKP6EHFKENA3VCVKFY
Phase: '1'
Track: governance-process
Implements: RFC-6
Blocked-on: —
---
# 50 — Decide whether Drift detected doubles as the new-ADR-needed trigger

## What

Decide whether a record's `Drift detected` state can double as the trigger for "this needs a new decision record, not just a revision-log entry," or whether that judgment call always needs a human.

## Why

RFC-6's revision log handles "the plan details changed, the decision still holds"; a new ADR handles "the decision itself must change." Nothing currently names what separates those two cases operationally -- this is arguably the hardest boundary in the whole record model, and it's currently unaddressed by any record.

## Done means

An ADR decides whether a record's `Drift detected` state can trigger a new-decision-needed signal or whether that judgment always needs a human, stating what separates a revision-log entry from a new ADR; if tooling is adopted it is a declared rule with a fixture, and `RFC-6` points at the decision.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |