---
Stable-Id: 01M6JKNZNG70E53R1KHPCY40MW
Status: Open
Found-in: 'a trial run over a real adopter corpus (2026-11-04)'
Regression-test: 'not yet written -- Status: Open, the fix has not been made'
---
# 185 — Config.scope-matches-nothing reports its file as an absolute path

## What was wrong

`config.scope-matches-nothing` reports its `file` as the absolute path to the configuration, while every other rule reports a repository-relative path. Minor: output differs by machine, so a finding cannot be compared or baselined across checkouts.

## Why nothing caught it

The rule's tests asserted the message, never the `file` field's form, so the one rule that built its path differently was not noticed.

## References

BUG-5 of the adopter switchover report.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this defect. | **substantive** |
