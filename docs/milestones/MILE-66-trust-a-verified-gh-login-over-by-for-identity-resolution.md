# 66 — Trust a verified gh login over --by for identity resolution

> Status: Done
> Stable-Id: 01M1YNNEQPK3PG0PZPG3K08QFG
> Phase: 0
> Track: accountability-identity
> Implements: ADR-31

## What

Identity resolution for `fix --apply` and `new` now checks a verified `gh api user` login first, then `git config user.name`, and only falls back to an explicit `--by` value when neither verified source is available -- reversing the previous priority, where a typed `--by` could silently override an authenticated session.

## Why

An unverified, typed name overriding a verified login is exactly backwards for a field whose entire purpose is accountability -- the strongest available identity signal should always win over the weakest.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-06 | Backfilled: shipped before milestone tracking existed. | **structural** |
