---
Stable-Id: 01M7KD3Q7HAT7GW9RBH15R0W6G
Status: Planned
Phase: '1'
Track: schema-governance
Blocked-on: null
---
# 324 — A value a type declares is not read as a placeholder or a pending marker

## What

Stop `field.quality` and `field.pending` from reporting a field's value as a placeholder or a pending marker when the configuration declares that value for that field, and have the load report the collision.

## Why

On 2026-11-17 an agent declared `statuses.values: [Todo, InProgress, Done, WontDo]` with `initial: [Todo]`. The configuration loaded clean, `urzua new` wrote `Status: Todo`, and `check` reported `field.quality`: "field 'Status' is Placeholder". Declaring `statuses.placeholders` without that word removed the finding and exposed a second, `field.pending`: "field 'Status' is marked pending". The agent renamed the status. A word the repository itself declared as a legitimate value of a field is not a placeholder in that field; the engine's placeholder and pending vocabularies are defaults, not a reason to forbid a status name. Declaring both `statuses.placeholders` and `statuses.pending` without the word avoids both findings (the reporting agent confirmed this on release 0.4.6 on 2026-11-17), but neither finding's message points to those keys.

## Done means

- A value declared for a field (a status value today; any declared field vocabulary) is never reported by `field.quality` as a placeholder or by `field.pending` as pending in that field, shown by a test with a status whose name is in both default vocabularies.
- The same word in a field that does not declare it is still reported.
- `urzua new` followed by `urzua check` on a type whose initial status is such a word reports no finding about the status.
- `urzua doctor` names a declared value that is also in the placeholder or pending vocabulary at severity `info`, so the overlap is visible, shown by a test.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Planned`. **Why:** Reported on 2026-11-17 by an agent configuring a new adopting repository on release 0.4.6, relayed at the maintainer's request. | **structural** |
