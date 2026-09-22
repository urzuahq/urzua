---
default: major
---

**Breaking: `FixReport.records_examined` and `MigrateSchemaReport.records_examined` are replaced by
`population`.** Both were the bare, hand-maintained `usize` the rest of the report contract moved away
from earlier in this release. `fix`'s population reports the `Embodiment`/`Realized-by` pair as a
declared slot (`Absent` when either is missing, matching `embodiment.consistency`'s own pair-slot
handling); `migrate schema --report`'s population reports a record whose header did not parse as
`Unreadable`.

Fixes a real false diagnosis: `migrate schema --report` classified a record with an unparsed header
as the candidate field being `Blank`, telling the operator the field would fail there specifically —
when the true defect is that nothing in the header is readable at all. Such a record is now excluded
from `would_fail` and disclosed instead as a `Notice`.
