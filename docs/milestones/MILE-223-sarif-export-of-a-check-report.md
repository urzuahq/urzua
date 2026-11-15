---
Stable-Id: 01M7FVT37BJYZSV692RXGCYZPN
Status: Planned
Phase: '2'
Track: interop
Blocked-on: null
---
# 223 — SARIF export of a check report

## What

Export a `check` report as SARIF so code-scanning interfaces show findings without the custom action.

## Why

the audit of 2026-11-15 found no SARIF output; an adopter who wants findings in a code-scanning view has to write the conversion.

## Done means

- A documented command writes a SARIF file from a `check` report, and the file validates against the SARIF 2.1.0 schema in a test.
- Each finding maps to a SARIF result with rule id, level, file and message, and waived findings are marked as suppressed.
- The CI reference page shows uploading the file.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
