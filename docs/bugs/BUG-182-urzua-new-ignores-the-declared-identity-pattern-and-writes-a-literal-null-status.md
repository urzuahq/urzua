---
Stable-Id: 01M6JKNXV24H9XAAQW3NFJ7D5S
Status: Open
Found-in: 'a trial run over a real adopter corpus (2026-11-04)'
Regression-test: pending
---
# 182 — Urzua new ignores the declared identity pattern and writes a literal null status

## What was wrong

`urzua new adr "x"` against a type declaring an identity pattern with a four-digit number writes a file named with an unpadded number, an H1 with the number but no type prefix, a literal `Status: null`, and a `Stable-Id` the type's own closed field set rejects unless declared. The file then fails `filename.identity-pattern` and `field.quality`. `MILE-134` says the generated target must follow the declared pattern; it does not.

## Why nothing caught it

No test ran `urzua new` and then `check` over the result for a type with a non-default pattern, so the generator and the checker were never compared.

## References

BUG-2 of the adopter switchover report. Related gap: the filed `urzua new` honouring the declared pattern milestone; `MILE-134`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this defect. | **substantive** |
