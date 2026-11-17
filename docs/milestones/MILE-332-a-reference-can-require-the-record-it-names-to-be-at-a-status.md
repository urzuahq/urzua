---
Stable-Id: 01M7KRAVSHV0JEQ15B52ZZ329T
Status: Planned
Phase: '1'
Track: schema-governance
Blocked-on: null
---
# 332 — A reference can require the record it names to be at a status

## What

Let a configuration declare, for a reference field, the statuses the named record must be at once the referring record is at given statuses, reported by the rule that already judges a target's status.

## Why

On release 0.4.7 an agent replaced one decision record with another: the new record names the old one in a field declared for that, and the old one was moved to its replaced status, both by hand. `urzua check` would have passed with the old record still at its accepted status. `pointer.target-status` can say which statuses a target must NOT be at; nothing can say which it must be at, or tie that to the referring record's own status. `relation.supersession-reciprocity` checks that the old record points back, not what status it holds. The half-done case, a replacement accepted while the record it replaces still reads as current, is a structural mistake no model is needed to see.

## Done means

- `pointer.target-status` accepts, per field, the statuses the target must be at, alongside the existing statuses it must not be at, and an optional condition on the referring record's own status, shown by a test in which a record at an accepted status names a target that is not at the declared status.
- The finding names both records, the field, the target's status and the statuses declared, and its suggested action is the status change on the target.
- A configuration that declares only the existing option reports exactly as before.
- The configuration reference shows the replaced-record case as its example, using words from a fixture configuration.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Planned`. **Why:** Reported on 2026-11-17 by an agent in an adopting repository on release 0.4.7, after it superseded one decision record with another, relayed at the maintainer's request. | **structural** |
