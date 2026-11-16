---
Stable-Id: 01M7HY8PW0XH7G9WSRV3E71B63
Status: Planned
Phase: '1'
Track: schema-governance
Blocked-on: null
---
# 304 — The override log's columns are declared, not fixed

## What

Let a configuration name the columns of the override log.

## Why

An audit on 2026-11-16 read all non-test engine code for places that name a particular record type, status, field or workflow. It found the override log's seven column names fixed in the rule that reads it.

## Done means

- The columns have a configuration key whose default is today's list; a log with other declared column names is read, shown by a test.
- A configuration that declares none behaves as before.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from the audit of 2026-11-16, asked for after the maintainer said the tooling has no opinion about which record types a repository defines and that this is driven by configuration, not by the code. | **structural** |
