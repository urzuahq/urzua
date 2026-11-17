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

Setting up a checklist for one type on release 0.4.7 took a configuration load error and reading a specification and a decision record, because `docs/reference` has one table row for the key and no example. The load error asked for the evidence field to be listed among the type's known fields although the item mechanism never reads that field. The configuration also refuses to load until the agreement rule is named under `rules:`, where the rules for declared field values and declared reference targets switch on by declaration. An adopting repository then adopted it from a tested example in about ten minutes with no load error, and reported on 2026-11-17 what still took guesses: citing a record took three tries because the shape error gives the path grammar and not the categories; a cited record could not satisfy an item until a tiers key the example omitted was added; and a decision record had to be cited under a category named for another kind of record. The mechanism is the deterministic answer to "is this record finished"; a repository that cannot find how to declare it writes its acceptance checks as prose instead.

## Done means

- A type can declare checklist items without listing an evidence field it does not use, shown by a test with the smallest declaration that loads.
- The rules that judge a declared checklist are on by declaration, like the other rules that act only on a declaration; a configuration that names them keeps the level it gives.
- The reference has a page for the checklist with a complete configuration block, a record with a met item, an unmet item and a waived item, the evidence forms accepted and those rejected, and what `urzua fix` does and does not change; the block and the record are loaded by a test.
- The load error for an incomplete checklist declaration names the missing key and shows the smallest valid block.
- A shape error on an evidence clause lists the categories the configuration accepts; a locator that names a record by its identifier either resolves or is told to use the record's path, and a locator that did not resolve says it was read as a path; a record of any declared type can be cited under a category that does not misname it; and a finding that a cited record's tier does not count names the key that would make it count, shown by a test of each.
- The reference page states that a record whose items carry no clause is not judged until it reaches a finished status, so a repository can convert its records one at a time.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Planned`. **Why:** Found on 2026-11-17 while setting up a checklist for an adopting repository's type on release 0.4.7, after the maintainer asked why that repository's unmet acceptance checks had not been caught without a model. | **structural** |
