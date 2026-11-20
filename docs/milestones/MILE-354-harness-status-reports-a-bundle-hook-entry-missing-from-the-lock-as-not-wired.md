---
Stable-Id: 01M7W0B0GHBSAKN5YPJ179YXHW
Status: Planned
Phase: '1'
Track: harness
Blocked-on:
---
# 354 — harness status reports a bundle hook entry missing from the lock as not wired

## What

The session-start check (`hook_entry_problems` in `rust/crates/urzua-cli/src/harness/run.rs`) reports a hook entry the
current bundle wants that the lock does not list, as it already reports a locked entry that is not wired. Today it walks
only the lock's entries, so a repository installed before a release that adds an entry (the two AskUserQuestion entries
of `ADR-105`) hears nothing at session start; `urzua harness status` does report it.

## Why

Found by an independent review of #612 on 2026-11-20. The failure is safe (without the question hooks the gated move
stays refused), but the person is not told why the question route never records an answer until they run
`urzua harness status`.

## Done means

- In a repository whose lock predates a bundle hook entry, the session-start check names that entry as not wired and says to run `urzua harness install`; shown by a binary test.
- A repository whose lock and settings match the bundle reports nothing new; shown by the existing session-start tests passing unchanged.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-20 | Filed. **Why:** an independent review of #612 found that the session-start check walks only the lock's entries. | **substantive** |
