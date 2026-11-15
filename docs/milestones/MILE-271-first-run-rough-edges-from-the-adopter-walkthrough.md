---
Stable-Id: 01M7FW744C1QEMS6YF6PAZ38VG
Status: Planned
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 271 — First-run rough edges from the adopter walkthrough

## What

Fix the smaller defects the walkthrough hit: silent no-ops, missing line numbers, unhelpful messages and a drafted waiver that fails its own check.

## Why

A walkthrough of 2026-11-15 followed only the user documentation with the released binary and recorded each of these as slowing a first adoption.

## Done means

- `migrate ids --apply` on a type with no header reports that it wrote nothing and why.
- A waiver drafted by `urzua waive` passes `urzua check` without editing the quoted finding text.
- Malformed frontmatter is reported with a line number, and a missing closing delimiter is named as such.
- `urzua doctor` does not report an error for a declared type whose directory `urzua new` will create.
- A misspelt rule id in the configuration gets a nearest-match suggestion and a line number.
- A bad `--base` reports which ref could not be resolved, without a raw git error.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
