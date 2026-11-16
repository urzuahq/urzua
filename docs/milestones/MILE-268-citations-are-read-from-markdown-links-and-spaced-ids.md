---
Stable-Id: 01M7FW6DEZPHXW1NYT2NGH4PA5
Status: Planned
Phase: '1'
Track: section-checks
Blocked-on: null
---
# 268 — Citations are read from markdown links and spaced ids

## What

Read a path cited as a markdown link target, and a record cited with a space or leading zeros, in the citation rules.

## Why

A walkthrough of 2026-11-15 followed only the user documentation with the released binary. A broken link to a source file was found only when rewritten as a backticked path, and a reference written as a type name, a space and a zero-padded number was not read as a reference, so a dangling supersession in the common ADR style passed.

## Done means

- A markdown link to a file that does not exist is a finding under the path citation rule.
- A configured reference form with a space and leading zeros resolves or is reported as dangling.
- A configuration that does not declare the extra forms behaves as before.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
