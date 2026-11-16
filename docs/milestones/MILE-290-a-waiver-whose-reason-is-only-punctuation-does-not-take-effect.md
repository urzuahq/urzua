---
Stable-Id: 01M7HW0JV72KQ2GD2JTRCQHEQM
Status: Planned
Phase: '1'
Track: schema-governance
Blocked-on: null
---
# 290 — A waiver whose reason is only punctuation does not take effect

## What

Treat a required waiver field that holds only a dash or other punctuation as blank, as the override log already does.

## Why

Found on 2026-11-16 while urzua's own builders and reviewers used the tool to land a week of pull requests. Two reviewers reproduced that a waiver with a reason of a single em dash takes effect and suppresses its finding, while the override log's check reads dashes as blank.

## Done means

- A waiver whose `Reason` is `—`, `-` or other punctuation with no letter or digit does not take effect and is reported, shown by a test.
- The override log and the waiver judgement share one definition of a blank value.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from friction met while using urzua on its own repository on 2026-11-16; the maintainer said that day that issues met in using and adopting the tool are improvement points. | **structural** |
