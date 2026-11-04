---
Stable-Id: 01M6HZ0MAXAFHSYZSBZ4D6KBAP
Status: Planned
Phase: '0'
Track: schema-governance
Implements: RFC-33
---
# 137 — Validate date fields and require the acceptance date exactly on accepted-type statuses

## What

Date-valued fields are validated as dates: well-formed (`YYYY-MM-DD`) and a real calendar date (no `2026-02-30`). A type also declares which statuses are accepted-type; the acceptance date field is required on those and forbidden (other than as a placeholder) on every other status. All violations are errors. The status-dependent requirement is expressed through the status-dependent required and forbidden fields in `MILE-132`; this milestone owns the date shape and the calendar check.

## Why

urzua checks that a date field is present but never that it is a date, so `2026-13-45` or free text passes. The adopter's linter requires its record date to be well-formed, requires its acceptance date to be a real calendar date, requires that date exactly when the record is accepted, superseded or deprecated, and rejects it on a record that is not. A replacement without these lets an unaccepted record carry an acceptance date, which reads as a decision nobody made.

## Done means

A date field declared as a date type rejects a malformed value and an impossible calendar date, each with an error; an accepted-type record without the acceptance date, and any other record carrying one, each produce an error, with a blank or declared placeholder tolerated on the forbidden side; the rule reports how many date values it examined.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the adopter's linter validates date shape, calendar validity and the acceptance date's presence and absence by status; urzua validates none of it. The check that the acceptance date is not earlier than the record date is consciously not carried over: the adopter itself treats it as usually legitimate. | **substantive** |
