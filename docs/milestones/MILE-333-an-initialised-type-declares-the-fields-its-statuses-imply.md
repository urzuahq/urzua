---
Stable-Id: 01M7KRB3T0BTMHFDCDQXWQJN5A
Status: Planned
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 333 — An initialised type declares the fields its statuses imply

## What

Make the profiles `urzua init --types` writes consistent with themselves: a profile whose statuses include a replaced state declares the field that names the replaced record, as a reference to its own type, and every profile declares both reference lists.

## Why

On release 0.4.7 `urzua init --types adr,...` wrote a type whose statuses include a replaced state, with no field to name the record replaced and `known_fields: []`. The first replacement added that field to a record and failed `header.field-set-consistency`. Declaring it as a reference then produced `config.pointer-declaration-missing`, which asks for the companion list as well: a second round trip for one field. The messages were clear; the profile simply did not declare what its own statuses need.

## Done means

- Each profile that declares a replaced state declares the field naming the replaced record and the field naming the replacement, as references to the same type with the relation between them required to be echoed, shown by a test that a first replacement in a freshly initialised repository checks clean.
- Each profile declares both reference lists, so adding a reference field produces no companion-declaration finding, shown by a test.
- A load-time notice names a status whose meaning a profile declares but whose fields it does not, for a hand-written configuration with the same gap.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Planned`. **Why:** Reported on 2026-11-17 by an agent in an adopting repository on release 0.4.7, after it superseded one decision record with another, relayed at the maintainer's request. | **structural** |
