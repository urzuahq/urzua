---
Status: Planned
Stable-Id: 01M2QB6RMF6Z3B5W1X8YCAES8T
Phase: '2'
Track: schema-governance
Implements: —
Blocked-on: MILE-98
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

*Update 2026-11-02:* `RFC-33` is now Accepted, so the decision half of this dependency is met; what remains is building the model, tracked as `MILE-98`.

Not urgent. `MILE-13` can say `Blocked-on: RFC-9` today and keep the "Q2" detail in its body -- a
resolvable reference plus prose beats prose alone, and `BUG-39` is the cheaper half regardless.

## Done means

A reference can name part of a record (`RFC-9#open-questions`), the form is parsed by one shared reference reader that `pointer.resolution` and `narrative-field.stale` use, a reference to a section that does not exist is reported, and `MILE-13`'s `Blocked-on` is rewritten as `RFC-9#...` instead of prose around a reference; it builds on `MILE-98`'s sections and is exercised in urzua's own milestones.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Filed. **Why:** `MILE-13` wanted to depend on one question inside `RFC-9` and had no way to say so. Recorded as a schema gap rather than a formatting lapse, and blocked on the document model, since an anchor into a record means nothing until sections exist. | **substantive** |
> | 2026-09-18 | `Blocked-on` now names `MILE-98`. **Why:** the declared document model had no milestone -- `MILE-4` was marked absorbed into `RFC-33` and the work moved into an RFC, so six records were blocked on something the plan did not track. Naming it makes the dependency resolvable, and `narrative-field.stale` can report when it moves. | **structural** |
> | 2026-11-02 | Unblocked: `RFC-33` was Accepted; maintainer decision 2026-11-02. `Blocked-on` now reads `MILE-98`; `Status` stays `Planned`. **Why:** `narrative-field.stale` flagged the reference once `RFC-33` reached a terminal status, and the maintainer decided the dependency no longer holds. `MILE-98` is still open and stays named. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
