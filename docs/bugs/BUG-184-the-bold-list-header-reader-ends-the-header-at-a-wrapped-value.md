---
Stable-Id: 01M6JKNZ1F635Y5F0NEPWB3FWC
Status: Open
Found-in: 'a trial run over a real adopter corpus (2026-11-04)'
Regression-test: 'not yet written -- Status: Open, the fix has not been made'
---
# 184 — The bold-list header reader ends the header at a wrapped value

## What was wrong

The bold-list header reader ends the header at a wrapped value (an indented continuation line), so every field after it reads as absent. On a real adopter corpus not yet migrated this produced over a hundred false `status.required-field-missing` findings on one type and a dozen more on another, which makes an as-is run untrustworthy. Full removal of the blockquote and bold-list readers (`MILE-85`) resolves it; it is recorded so the interim state is not mistaken for a real finding.

## Why nothing caught it

Fixtures kept every bold-list field on one line, so no test wrapped a value.

## References

BUG-4 of the adopter switchover report. Resolved by `MILE-85`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this defect. | **substantive** |
