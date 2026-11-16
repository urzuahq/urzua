---
Stable-Id: 01M7HY7AFEXPX56KCXCPYA6T1A
Status: Planned
Phase: '1'
Track: roadmap-tracking
Blocked-on: null
---
# 301 — Reports can group by a related record, not only by a field's text

## What

Let the grouping of the planning report key on the record a relation points at, so each group carries that record's id, state and counts, and a type used only for grouping is not dropped from the report.

## Why

An audit on 2026-11-16 read all non-test engine code for places that name a particular record type, status, field or workflow. It found that grouping keys on a header field's text value, so a group carries nothing about the record it names, and a parent type left out of the declared work types disappears from the groups.

## Done means

- A configuration groups the report by a declared relation, and each group names the target record with its state and the number of members in each state, shown by a test with two invented types.
- A type declared only as a grouping target appears as a group and not as work.
- Grouping by a field's text still works as before.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from the audit of 2026-11-16, asked for after the maintainer said the tooling has no opinion about which record types a repository defines and that this is driven by configuration, not by the code. | **structural** |
