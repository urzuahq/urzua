---
Stable-Id: 01M6JW438QHFRMZPF22SZM9JAM
Status: Open
Found-in: 'a re-run of urzua over a real adopter corpus (2026-11-04)'
Regression-test: 'planned: a test that fails before the fix and passes after, plus the same case exercised through urzua check in this repository'
---
# 188 — Urzua new writes an H1 id that does not follow the declared identity pattern's padding and drops a template's non-required header fields

## What was wrong

Against a type whose identity pattern declares zero-padded numbers, `urzua new` writes a padded filename but an unpadded H1 id beside it, so the two disagree. It also drops header fields a type's template declares that are not required, so the new record starts without fields the template meant it to carry. `BUG-182` fitted the file name and H1 to the declared pattern and `MILE-151` made the generator follow the declared field set; neither covered the H1 padding nor the template's non-required fields.

## Why nothing caught it

The `BUG-182` tests ran `urzua new` then `check` and passed because the unpadded H1 satisfies the checker; none compared the H1 with the file name, and none used a template carrying optional header fields.

## References

Related: `BUG-182`, `MILE-151`. Done when `urzua new` writes an H1 id padded exactly as the file name is, keeps the template's non-required header fields, and this is exercised by a type with a padded pattern and an optional template field in urzua's own `.urzua/config.yaml`, with `urzua check` clean over the result.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this defect. | **substantive** |
