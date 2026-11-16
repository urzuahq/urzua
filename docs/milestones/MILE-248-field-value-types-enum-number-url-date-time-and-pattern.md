---
Stable-Id: 01M7FW00K5R5HBDA003B6XWBY4
Status: Planned
Phase: '2'
Track: engine-extensibility
Blocked-on: null
---
# 248 — Field value types: enum, number, URL, date-time and pattern

## What

Let a record type declare the type of a field's value (an enumerated list, a number with a range, a URL, a date-time, a pattern) and check it with one rule.

## Why

the audit of 2026-11-15's extensibility experiments found that a field is a name plus presence. `Severity: SEV7`, `Accuracy: 1.7` and a misspelt enum value all pass. This blocked more workflows than any other limit: postmortems, compliance controls, experiment logs.

## Done means

- A decision record states the declaration and the rule.
- A record whose field value is outside its declared enum, range or pattern gets a finding naming the field, the value and what is allowed.
- A configuration that declares no field types behaves as before.
- The capability is exercised in urzua's own `.urzua/config.yaml` and corpus, and `urzua check` reports 0 unwaived findings.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
