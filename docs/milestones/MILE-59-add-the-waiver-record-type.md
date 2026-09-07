# 59 — Add the waiver record type

> Status: Done
> Stable-Id: 01M1YNNAYPRGKFY23FMFWCQ907
> Phase: 0
> Track: accountability-identity
> Implements: ADR-11

## What

The `waiver` record type: a reviewed exception to a rule is its own record (`Rule`, `Scope`, `Reason`, optional `Expires`), never a config-level ignore list. A waived finding stays listed, just excluded from the blocking exit code.

## Why

An ignore list is unreviewable and grows forever; a waiver record is a normal record with an author, a reason, and an expiry -- reviewable in the same diff as everything else.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-04 | Backfilled: shipped before milestone tracking existed. | **structural** |
