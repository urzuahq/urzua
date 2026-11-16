---
Stable-Id: 01M7FVWTXJQYXYT63ZYTN071HP
Status: Planned
Phase: '1'
Track: schema-governance
Blocked-on: null
---
# 235 — Output and network values are read from configuration

## What

Move the hard-coded output and network values into configuration: the API host, timeouts, page size and page cap of `refs refresh`, the identity lookup timeout, and the number of items a message lists before it truncates.

## Why

`ADR-101`'s amendment of 2026-11-15. the audit of 2026-11-15 found thirteen such values; the API host could be changed only by an environment variable.

## Done means

- Each value has a config key whose default is today's value; a configuration that declares none behaves as before, shown by a test.
- The protocol and proxy restrictions of `refs refresh` and its page cap are security floors that configuration cannot weaken, shown by a test.
- `SPEC-3` lists the request headers and the next-action order as format invariants with their reasons.
- The capability is exercised in urzua's own `.urzua/config.yaml` and corpus, and `urzua check` reports 0 unwaived findings.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
