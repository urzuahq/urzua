---
Stable-Id: 01M7K8MP4K7PYCQ3FGJMQD0AG0
Status: Planned
Phase: '1'
Track: write-path
Blocked-on: null
---
# 318 — Re-pinning many records is one call and one pass

## What

Let one `urzua fix --rebaseline --apply` re-pin every stale record that a change to named files caused, with one reason per changed file, and repeat until nothing it wrote is stale.

## Why

On 2026-11-16 three builders each re-pinned between 23 and 62 records by one call per record; 35 took more than two minutes. A re-pin row written into a record that other records pin makes those stale in turn, and the output does not say a second pass is needed; formatting a file after re-pinning makes every record that pins it stale again. The reason is per changed file, not per record, yet it is typed once per record.

## Done means

- One invocation accepts a reason per changed file and re-pins every record whose stale pins are all explained by those files, shown by a test with ten records and two files.
- The invocation repeats for records made stale by its own rows and reports how many passes it made; the cascade reason names the row that caused it.
- A record with a stale pin on a file no reason was given for is left alone and listed with a `next_action`.
- The run on this repository's own records for a one-file change finishes in a time recorded in the pull request, against the per-record time before.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Met on 2026-11-16 by a builder using the tool on this repository's own records; the maintainer said that day that friction met using the tool is an improvement point. | **structural** |
