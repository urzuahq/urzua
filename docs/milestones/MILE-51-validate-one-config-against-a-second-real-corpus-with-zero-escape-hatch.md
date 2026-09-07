# 51 — Validate one config against a second real corpus with zero escape hatch

> Status: Planned
> Stable-Id: 01M1YN87AP04NCFMXY4WBJK4EK
> Phase: 1
> Track: corpus-corrections

## What

Run `urzua init` + `check` against a second real, independently-authored corpus, with the explicit success criterion that the same config mechanism (record types, required fields, header shapes) expresses both corpora's real rules with no escape hatch or code change.

## Why

This repo's own corpus is the only one `check` has ever run against. "One config, not a fork per repo" is a real, falsifiable claim about the schema, and it's never actually been tested against a corpus this project didn't author -- the cheapest, most informative validation available before building anything else on top of the current schema.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
