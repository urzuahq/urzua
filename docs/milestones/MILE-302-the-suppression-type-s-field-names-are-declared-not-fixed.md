---
Stable-Id: 01M7HY7TRBK5D74FP41XBFWH5D
Status: Planned
Phase: '1'
Track: schema-governance
Blocked-on: null
---
# 302 — The suppression type's field names are declared, not fixed

## What

Let a configuration name the fields a suppression record uses for the rule it suppresses, its scope and its expiry, as it already can for the reason and the owner.

## Why

An audit on 2026-11-16 read all non-test engine code for places that name a particular record type, status, field or workflow. It found `Rule`, `Scope`, `Expires`, `Variant` and `Key-path` fixed as the header field names of any suppression type, with the configuration forbidden to rename them; the reason and owner fields can already be renamed.

## Done means

- Each of those field names has a configuration key whose default is today's name, and a suppression type declared with other names suppresses findings, shown by a test.
- A configuration that declares none behaves as before.
- `SPEC-3` no longer lists these names as format invariants.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from the audit of 2026-11-16, asked for after the maintainer said the tooling has no opinion about which record types a repository defines and that this is driven by configuration, not by the code. | **structural** |
