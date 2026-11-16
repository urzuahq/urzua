---
Stable-Id: 01M7K3J2ZJSYMP02F321B22VH0
Status: Planned
Phase: '1'
Track: write-path
Blocked-on: null
---
# 310 — A re-pin works on a record that has no revision log yet

## What

Make `urzua fix --rebaseline --apply` succeed on a record without a revision log by creating the log in the declared shape, or refuse with a runnable next action that does.

## Why

On 2026-11-16, in an adopting repository, a re-pin of a record with a stale pin was refused because the record had no revision log section, and the log had to be written by hand before the tool would add its row. In that repository 510 of 529 records of the type have no log, so nearly every first re-pin meets this. A hand-written log section is also exactly the edit that `diff.pin-changed-unattested` exists to distinguish from a tool-written row.

## Done means

- `urzua fix --rebaseline --apply` on a record with a stale pin and no revision log writes the log section in the shape the configuration declares, with the re-pin row as its first row, shown by a test.
- Where the record has no section the configured log identity line can follow (no body sections at all), the refusal names the missing section and carries a `next_action` that creates it.
- The row written this way satisfies `diff.pin-changed-unattested` under `check --base`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Met on 2026-11-16 while re-pinning records in an adopting repository; the maintainer said that day "when you're seeing issues with like the use of the tool and how we adopt the tool, these are improvement points for us". | **structural** |
