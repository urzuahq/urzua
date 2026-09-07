# 43 — Write-time reciprocity auto-linking when a record supersedes another

> Status: Planned
> Stable-Id: 01M1YN835FM8NATNCKA34F6FED
> Phase: 1
> Track: governance-process

## What

When `urzua new`/`fix` writes a `Supersedes`/`Superseded-by` claim, write the reciprocal pointer on the target record automatically, instead of relying on a human to author both sides and `relation.supersession-reciprocity` to catch it after the fact.

## Why

`relation.supersession-reciprocity` already detects a one-directional supersession claim -- real, useful, but reactive. Auto-linking at write time prevents the defect from ever existing instead of catching it on the next `check` run, the same shift from detect to prevent this project already made for other write paths (`fix`'s gated apply mode, ADR-14's revision log).

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
