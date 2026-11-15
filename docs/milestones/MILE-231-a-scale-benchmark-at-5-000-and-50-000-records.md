---
Stable-Id: 01M7FVVXHQ4E6HDKD407R3808A
Status: Planned
Phase: '2'
Track: scale-and-analytics
Blocked-on: null
---
# 231 — A scale benchmark at 5,000 and 50,000 records

## What

Generate synthetic corpora of 5,000 and 50,000 records and measure every command on them, recording time and memory.

## Why

the audit of 2026-11-15 found no measurement beyond this repository's own 594 records, where a full check already takes ten seconds.

## Done means

- A script generates a corpus of a requested size with realistic references and evidence.
- The measured time and peak memory of `check`, `check --base`, `graph`, `roadmap` and `index` at both sizes are recorded with the version measured.
- Any command that is worse than linear in the number of records is named, with the cause.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
