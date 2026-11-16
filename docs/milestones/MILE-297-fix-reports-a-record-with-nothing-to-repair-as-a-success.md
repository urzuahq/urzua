---
Stable-Id: 01M7HW2T9JEHPBZB5SREN09MMN
Status: Planned
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 297 — fix reports a record with nothing to repair as a success

## What

Make `urzua fix --apply --ids <record>` on a record that already agrees with its evidence report that plainly, and correct the wording of the tool-written row.

## Why

Found on 2026-11-16 while urzua's own builders and reviewers used the tool to land a week of pull requests. Re-running `fix --apply --ids` on a record that needs nothing exits with a partial-failure status, which reads as an error to an agent; and the row the tool writes says a record "computes to Verified" beside a computed value of Done.

## Done means

- `urzua fix --apply --ids <record>` on a record with no pending repair exits 0 with a status that says nothing needed doing.
- The row the tool writes for a status repair states one computed value, consistently.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from friction met while using urzua on its own repository on 2026-11-16; the maintainer said that day that issues met in using and adopting the tool are improvement points. | **structural** |
