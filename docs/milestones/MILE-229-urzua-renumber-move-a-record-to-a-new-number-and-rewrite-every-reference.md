---
Stable-Id: 01M7FVVED342V3KFQZM73CHFCS
Status: Planned
Phase: '1'
Track: write-path
Blocked-on: null
---
# 229 — urzua renumber: move a record to a new number and rewrite every reference

## What

Add a command that changes a record's number (or resolves a duplicate), renames the file, rewrites its heading and rewrites every reference to it across the corpus and in source back-pointers, with a dry run.

## Why

An agent working in an adopting repository reported that urzua has no renumber command: when two branches take the same number, or a record must move, every reference has to be found and edited by hand.

## Done means

- `urzua renumber <record> --to <n>` without `--apply` lists every file and line it would change, and with `--apply` makes those changes.
- After a renumber, `urzua check` reports no dangling reference and no duplicate number, shown by a test on a corpus with references in records and in source comments.
- The record's `Stable-Id` is unchanged, so `check --base` pairs the old and new paths as a rename.
- A `sequence` finding for a duplicate number suggests the command as its next action.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
> | 2026-11-16 | The renumbering will be delivered as part of `urzua fix --conflicts` rather than a separate `renumber` command. The Done-means text is kept as written. **Why:** maintainer decision of 2026-11-16: asked "Should fixing conflicts after a merge be one command under `fix`? It would do both jobs in one run: give a record whose number is now taken the next free number (renaming the file and rewriting every reference to it), and resolve records that conflict only in their evidence pins. Anything it can't safely fix, it lists and leaves alone.", answered "`urzua fix --conflicts`", having said "Maybe there's one go-to command that fixes this whole class of problems of conflicts... Maybe this is just what the fix command should be used for." | **structural** |
