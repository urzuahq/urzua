---
Stable-Id: 01M7FVWMDBGYQCMR35VG4QEDFT
Status: Planned
Phase: '1'
Track: schema-governance
Blocked-on: null
---
# 234 — Record authoring values are read from configuration

## What

Move the hard-coded values of `new`, `waive`, `reverify` and `migrate` into configuration: the waiver draft heading, the title length cut, the retry count, the stale-lock age and the scan size.

## Why

`ADR-101`'s amendment of 2026-11-15. the audit of 2026-11-15 found ten such values in the authoring commands.

## Done means

- Each value has a config key whose default is today's value; a configuration that declares none behaves as before, shown by a test.
- `SPEC-3` lists the authoring values that stay fixed as format invariants (the date format, the claim ref namespace, the wording `reverify` writes and parses back) with their reasons.
- The capability is exercised in urzua's own `.urzua/config.yaml` and corpus, and `urzua check` reports 0 unwaived findings.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
