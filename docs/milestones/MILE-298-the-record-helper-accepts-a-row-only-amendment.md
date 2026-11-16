---
Stable-Id: 01M7HW34E7BBWJNTT674AMHP7Z
Status: Planned
Phase: '1'
Track: write-path
Blocked-on: null
---
# 298 — The record helper accepts a row-only amendment

## What

Let the amendment helper finish an edit that only adds a revision-log row, such as recording that the maintainer accepted an amendment.

## Why

Found on 2026-11-16 while urzua's own builders and reviewers used the tool to land a week of pull requests. Recording an acceptance is one added log row; the amendment helper refused it because no amendment section was added, so the check had to be run by hand.

## Done means

- The amendment helper accepts an edit that adds only revision-log rows and runs `urzua check` on the record.
- The generated command's text says so.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from friction met while using urzua on its own repository on 2026-11-16; the maintainer said that day that issues met in using and adopting the tool are improvement points. | **structural** |
