---
Stable-Id: 01M82JQFE3N10A9C8PJA58AWVD
Status: Planned
Phase: '1'
Track: harness
Blocked-on:
---
# 356 — harness status reports a bundle hook entry the lock does not list

## What

`urzua harness status` reports a hook entry the current bundle wants that `.urzua/harness.lock` does not list, as it reports a locked entry that is not wired. Today `status_entries` (`rust/crates/urzua-cli/src/harness/mod.rs`) walks only the lock's entries, so a repository installed before a release that adds an entry (the two AskUserQuestion entries of `ADR-105`) shows nothing for it in `status`.

## Why

An independent review of #622 (the `MILE-354` branch) showed that `MILE-354`'s premise, that `status` already reports such an entry, was wrong; `MILE-354` made the session-start check report it. Making `status` report it changes the exit code and the outcome of a CI run for a repository with an older lock, which is a design call, so it is filed here rather than made in that PR.

## Done means

- In a repository whose lock predates a bundle hook entry that settings do not wire, `urzua harness status` reports that entry with a state that is not `ok`; shown by a binary test that fails without the change.
- An entry the lock does not list but settings already wire, a missing settings file, and a repository whose lock matches the bundle report nothing new; shown by binary tests.
- The decision on whether the new state changes `status`'s exit code is recorded in an ADR or in this milestone's revision log with the maintainer's quoted instruction.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
