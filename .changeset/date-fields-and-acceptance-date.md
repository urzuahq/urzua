---
default: minor
---

Two new opt-in rules validate dates. `date.field-valid` reports a value in a declared date field (`date_fields`) that is not a well-formed `YYYY-MM-DD` date or not a real calendar date, such as `2026-02-30`. `date.acceptance-by-status` requires the acceptance date (`acceptance_field`) on a record whose status is accepted-type (the type's declared `statuses.success`, or an `accepted_statuses` option) and reports it on any other status, tolerating a blank or placeholder there. Both report how many values or records they examined.
