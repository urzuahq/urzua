---
Stable-Id: 01M7KRXKYY2HZEK32QM3X2V3GQ
Status: Planned
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 335 — Declaring a checklist takes one block and the reference shows it

## What

Remove the requirements a checklist declaration has that it does not use, and add a reference page with a complete, tested example.

## Why

Setting up a checklist for one type on release 0.4.7 took a configuration load error and reading a specification and a decision record, because `docs/reference` has one table row for the key and no example. The load error asked for the evidence field to be listed among the type's known fields although the item mechanism never reads that field. The configuration also refuses to load until the agreement rule is named under `rules:`, where the rules for declared field values and declared reference targets switch on by declaration. The mechanism is the deterministic answer to "is this record finished"; a repository that cannot find how to declare it writes its acceptance checks as prose instead.

## Done means

- A type can declare checklist items without listing an evidence field it does not use, shown by a test with the smallest declaration that loads.
- The rules that judge a declared checklist are on by declaration, like the other rules that act only on a declaration; a configuration that names them keeps the level it gives.
- The reference has a page for the checklist with a complete configuration block, a record with a met item, an unmet item and a waived item, the evidence forms accepted and those rejected, and what `urzua fix` does and does not change; the block and the record are loaded by a test.
- The load error for an incomplete checklist declaration names the missing key and shows the smallest valid block.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Planned`. **Why:** Found on 2026-11-17 while setting up a checklist for an adopting repository's type on release 0.4.7, after the maintainer asked why that repository's unmet acceptance checks had not been caught without a model. | **structural** |
