---
Stable-Id: 01M7KB8P5NNR17N4JW7VWZN8N8
Status: Planned
Phase: '1'
Track: schema-governance
Blocked-on: null
---
# 319 — A requirement can depend on any field's value, not only the status

## What

Let a type declare that fields are required or forbidden when another declared field holds given values, with the same rule that judges the status-keyed form today.

## Why

`statuses.require` and `statuses.forbid` take `when:` as a list of status values and nothing else (`status_model.rs`). A repository that keeps one type with a field distinguishing its kinds cannot say that one kind must leave a pointer empty or must carry an extra field; its only route is a separate record type per kind, which is a modelling choice the configuration should not force.

## Done means

- A requirement entry can name a field and the values of it that trigger the entry, alongside or instead of status values, shown by a test where a field is required for one value and forbidden for another.
- The finding names the triggering field and value, and the existing status-keyed entries keep their meaning without a configuration change.
- A condition on a field the type does not declare, or on a value outside that field's declared vocabulary where one exists, is a load error.
- The configuration reference shows one example of each form.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Reported on 2026-11-16 by an agent setting up a new adopting repository on release 0.4.6, relayed at the maintainer's request; checked against the code the same day. | **structural** |
