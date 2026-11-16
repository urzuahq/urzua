---
Stable-Id: 01M7HW0XHAG5CE132HQNPPBH6G
Status: Planned
Phase: '1'
Track: embodiment-model
Blocked-on: null
---
# 291 — Spec versions and log rows do not collide between parallel pull requests

## What

Stop two pull requests that each amend the same spec from colliding on its version number and on where their log rows go.

## Why

Found on 2026-11-16 while urzua's own builders and reviewers used the tool to land a week of pull requests. Every feature pull request appended a section and a version bump to the same three specs, so each merge forced every other open pull request to renumber its versions by hand (one spec moved from 0.145 to 0.153 in two days, renumbered in four branches), and re-pin rows written mid-log made `urzua fix --conflicts` refuse 28 of 29 records in one merge.

## Done means

- Two branches that each add a section and a log row to the same spec merge with one documented command and no hand renumbering, shown by a test.
- `urzua fix --conflicts` resolves a record whose two sides each appended rows when one side's rows were not last in the log.
- The contributor documentation says how a version is assigned when two pull requests amend one spec.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from friction met while using urzua on its own repository on 2026-11-16; the maintainer said that day that issues met in using and adopting the tool are improvement points. | **structural** |
