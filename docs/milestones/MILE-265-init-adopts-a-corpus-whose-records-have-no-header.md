---
Stable-Id: 01M7FW5MSSZVTYPFH2JT42FN3D
Status: Planned
Phase: '1'
Track: header-format
Blocked-on: null
---
# 265 — init adopts a corpus whose records have no header

## What

Make `urzua init` propose a working configuration for a corpus in the common ADR style (numbered files, a title heading, status and context sections, no header block), and stop `doctor` sending the user back to `init`.

## Why

A walkthrough of 2026-11-15 followed only the user documentation with the released binary. `init` refused a fifteen-record corpus in the most common ADR style because no file has a header, and `doctor` then advised running `init`, a loop. The whole configuration had to be written by hand.

## Done means

- `urzua init` on such a corpus proposes a type with the sections it found as required sections, and `urzua check` then runs.
- `urzua doctor` after a refused `init` names the next step that works.
- `urzua init` in an empty repository names `--types`.
- An index page in a record directory is not proposed or read as a record.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
