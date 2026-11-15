---
Stable-Id: 01M7FW12QQ8ETT2SZ3A9V0Z5VT
Status: Planned
Phase: '2'
Track: engine-extensibility
Blocked-on: null
---
# 252 — Clock rules and more date forms

## What

Add rules that compare a date field with today (not in the future, overdue unless in a given status) and accept configured date and date-time forms.

## Why

the audit of 2026-11-15's extensibility experiments found that dates are `YYYY-MM-DD` only and nothing compares a field with today: a future acceptance date and a past due date both pass, and a comment period cannot be enforced.

## Done means

- A record with a date field in the future gets a finding when the rule is enabled.
- A record past its due date and not in an exempt status gets a finding.
- A type that declares another date form or a date-time has its values accepted and ordered correctly.
- Both rules honour `--as-of`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
