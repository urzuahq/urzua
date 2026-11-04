---
default: minor
---

Add `date.field-order`, an opt-in rule that checks one date field of a record is not later than another. List entries as `date_order: [{earlier: Date, later: Accepted-on}]`; a record whose first date is later than its second is an error, equal dates pass, and a value that is absent, a placeholder or not a real date is left to `date.field-valid` and the field-quality rules.
