---
Stable-Id: 01M7HY862WJPB4HJ6PHWCQGC9A
Status: Planned
Phase: '1'
Track: schema-governance
Blocked-on: null
---
# 303 — The identity field's name is declared, not fixed

## What

Let a configuration name the field that carries a record's stable identity.

## Why

An audit on 2026-11-16 read all non-test engine code for places that name a particular record type, status, field or workflow. It found the field name `Stable-Id` fixed in discovery, in record creation, in the migrate commands, in export and in the rules that pair a renamed file with its old copy.

## Done means

- The identity field has a configuration key whose default is today's name; a repository that declares another name gets rename pairing, creation and migration on that field, shown by a test.
- A configuration that declares none behaves as before.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from the audit of 2026-11-16, asked for after the maintainer said the tooling has no opinion about which record types a repository defines and that this is driven by configuration, not by the code. | **structural** |
