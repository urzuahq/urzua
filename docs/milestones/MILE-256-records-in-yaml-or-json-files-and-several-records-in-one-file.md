---
Stable-Id: 01M7FW25CK442FHA6H6S4GAS5X
Status: Planned
Phase: '3'
Track: header-format
Blocked-on: null
---
# 256 — Records in YAML or JSON files, and several records in one file

## What

Read a record from a YAML or JSON file, and more than one record from a single file.

## Why

the audit of 2026-11-15's extensibility experiments found that such files are ignored with no finding, so a register kept as one data file per record cannot be governed.

## Done means

- A decision record states the model for a record that is not a markdown file.
- A type declared as YAML has its files validated by the same rules.
- A file in a record directory that no type can read is reported, not skipped silently.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
