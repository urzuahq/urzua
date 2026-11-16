---
Stable-Id: 01M7KB94Y8JXS44KYVV2VB0MRZ
Status: Planned
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 321 — check has a compact report

## What

Add a report form for `check` that carries the status, counts, findings, notices as counts by kind, and the next action, and leaves out per-rule detail unless asked.

## Why

A clean `check` of one record in a freshly initialised repository prints 549 lines (11.8 KB), 505 of them `rules_executed` entries and a third of those `not-enabled`. In this repository a clean run is 1,209 lines (53 KB): 793 lines of per-rule entries, 257 of notices, 130 of suppressed findings. The readers are agents paying for every line, and the reporting agent wrote a script to extract the five fields it needed. The only option that reduces the per-rule section is `--rules`. The compact form is still JSON for agents, not a human rendering.

## Done means

- A flag, and a configuration default for it, selects the compact form: status, counts of files and findings by severity, the findings themselves, notices as counts by kind with one example each, suppressed findings as counts (`MILE-209`), and the next action, shown by a test that the compact report of a clean run in a freshly initialised repository is under a stated size.
- The full form remains available and unchanged in shape, and the compact form says how to get it.
- Rules that are not enabled appear in the compact form as one count.
- The hooks and the agent harness use the compact form where they print a report.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Reported on 2026-11-16 by an agent setting up a new adopting repository on release 0.4.6, relayed at the maintainer's request; checked against the code the same day. | **structural** |
