---
Status: Planned
Stable-Id: 01M2QB6RMF6Z3B5W1X8YCAES8T
Phase: '2'
Track: schema-governance
Implements: —
Blocked-on: RFC-33
---
# 97 — A reference into a record, not only at it

## What

A way to name part of a record: `RFC-9#open-questions` rather than `RFC-9`. Today every reference is
record-atomic, so a dependency on one section of a long record can only be written as prose.

## Why

`MILE-13`'s `Blocked-on` reads `RFC-9's own Q2`. That is not sloppiness -- it is the only way to say
what is true, because the dependency is on one question inside `RFC-9`, and the schema has no way to
express it. The author wrote prose around a reference, which is exactly what makes it invisible to
every rule (`BUG-39`).

The same capability is now wanted from three directions, which is the argument for it:

- **MADR** needs `###` addressable to check that each considered option has a pros-and-cons
  subsection.
- **This corpus** has `###` headings in 18 of 84 records that `check` cannot see at all.
- **This record** wants to point at one.

## Blocked on the document model

A reference into a record is meaningless until sections are declared and addressable, which is
`RFC-33`'s layer 1 (`sections.from`, with `depth` and `items`). The anchor syntax is the small part;
the model underneath it is the work.

Not urgent. `MILE-13` can say `Blocked-on: RFC-9` today and keep the "Q2" detail in its body -- a
resolvable reference plus prose beats prose alone, and `BUG-39` is the cheaper half regardless.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Filed. **Why:** `MILE-13` wanted to depend on one question inside `RFC-9` and had no way to say so. Recorded as a schema gap rather than a formatting lapse, and blocked on the document model, since an anchor into a record means nothing until sections exist. | **substantive** |
