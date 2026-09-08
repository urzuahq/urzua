# 50 — Decide whether Drift detected doubles as the new-ADR-needed trigger

> Status: Planned
> Stable-Id: 01M1YN86SKP6EHFKENA3VCVKFY
> Phase: 1
> Track: governance-process
> Implements: RFC-6
> Blocked-on: —

## What

Decide whether a record's `Drift detected` state can double as the trigger for "this needs a new decision record, not just a revision-log entry," or whether that judgment call always needs a human.

## Why

RFC-6's revision log handles "the plan details changed, the decision still holds"; a new ADR handles "the decision itself must change." Nothing currently names what separates those two cases operationally -- this is arguably the hardest boundary in the whole record model, and it's currently unaddressed by any record.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
