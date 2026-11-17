---
Stable-Id: 01M7KB8XERSKAE2T8PMYAWEC6J
Status: Planned
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 320 — A report whose every record was skipped does not read as ok

## What

When `check` examined no record because every candidate under the declared directories was skipped, give the report a top-level status that says so.

## Why

On 2026-11-16 an agent created six records with `urzua new`, ran `urzua check`, and got `"status": "ok"` with `files_examined: 0`: the records were untracked. The cause is reported, as an info notice and as the next action, but under a top-level `ok` in a report of several hundred lines, and the agent caught it only from the count. The `ok` is the designed empty-corpus status (`MILE-206`); a status is what a caller branches on, so the design is what this changes.

## Done means

- When at least one record file exists under a declared directory and none was examined, the top-level status is not `ok`; it names the state, and the exit code is documented, shown by a test with untracked records.
- A repository with no record files at all keeps its present status.
- The next action is unchanged: the command that stages the records, or the configuration key that includes untracked files.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Reported on 2026-11-16 by an agent setting up a new adopting repository on release 0.4.6, relayed at the maintainer's request; checked against the code the same day. | **structural** |
