---
default: minor
closes: []
---

Add `date.field-valid`, an opt-in rule for date-valued header fields. List the fields in `date_fields`; a value that is not a well-formed `YYYY-MM-DD` date, or names a day that does not exist such as `2026-02-30`, is an error, and the report states how many values were examined. Requiring an acceptance date on accepted-type statuses and forbidding it elsewhere is declared on the record type with `statuses.require` and `statuses.forbid` (see the configuration spec).
