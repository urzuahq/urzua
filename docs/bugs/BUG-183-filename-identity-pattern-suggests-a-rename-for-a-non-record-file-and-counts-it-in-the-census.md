---
Stable-Id: 01M6JKNYCZFWAQJ90KRWT13CBJ
Status: Open
Found-in: 'a trial run over a real adopter corpus (2026-11-04)'
Regression-test: pending
---
# 183 — Filename.identity-pattern suggests a rename for a non-record file and counts it in the census

## What was wrong

`filename.identity-pattern` suggests renaming `README.md` to `SPEC-NNNN-README.md` and counts the README in its "none of the record type's N files match" total, treating an index file as a record that failed to match.

## Why nothing caught it

No fixture put a non-record file in a record directory, so the rule's file census was never tested against a file that is not a record. Related gap: the declaration excluding non-record files.

## References

BUG-3 of the adopter switchover report.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this defect. | **substantive** |
