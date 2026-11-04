---
default: minor
---

Add `required` and `exempt_statuses` under a record type's `sections` declaration, and two opt-in
rules that read them. `section.required` reports a `##` section a record of the type must carry and
does not. `section.shape` checks a present section's content shape: `y-statement` requires the five
phrases `In the context of`, `facing`, `we decided`, `to achieve` and `accepting` in one block
(opened by an optional `marker` such as `TL;DR`), and `table-min-rows` requires a table with at
least `min_rows` data rows. Records in an `exempt_statuses` status, such as `Withdrawn`, are counted
out of scope. A type that declares no `required` is unchanged, and both rules report their population
in a new `section` unit.
