# 41 — Add a code-path binding field (affects) to the core schema

> Status: Planned
> Stable-Id: 01M1YN81YFRMVT2EWYM4PCKP5X
> Phase: 1
> Track: schema-governance
> Implements: RFC-1

## What

Add an `affects` field to the core schema so a decision record can declare which code paths its outcome touches -- a path/glob list, forward-looking, distinct from `Realized-by`'s backward-looking evidence.

## Why

Every relationship the schema has today points backward: a record cites its evidence (`Realized-by`), or another record (`Implements`/`Derives-from`). Nothing lets a decision declare what it touches going forward, so there's no way to flag "this decision's affected path just changed, has anyone re-checked it still holds" -- exactly the kind of drift `embodiment.consistency` already catches for evidence, but for the decision's own stated scope of effect.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
