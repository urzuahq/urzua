---
default: minor
---

A new opt-in rule, `date.field-valid`, reports a value in a declared date field (`date_fields`) that is not a well-formed `YYYY-MM-DD` date or not a real calendar date, such as `2026-02-30`, and reports how many values it examined. To require an acceptance date on accepted-type statuses and forbid it elsewhere, declare it with `statuses.require` / `statuses.forbid` on the record type; `SPEC-3` shows the pattern.
