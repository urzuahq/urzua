---
Stable-Id: 01M7N42FPM2HYBHZNXJ0Y1864Y
Status: Planned
Phase: '1'
Track: accountability-identity
Blocked-on:
---
# 342 — fix --conflicts resolves a record whose pinned file was merged by hand, and names the conflicted records when run without ids

## What

`urzua fix --conflicts` resolves a conflicted record when the file one of its pins names was itself merged, and when it is run with `--apply` and no ids it names the conflicted records in its answer.

## Why

On 2026-11-17 four pull requests merged main after other work had landed. `urzua fix --conflicts` resolved the records whose pinned files merged cleanly and refused the rest: thirteen records in one merge, eleven in another, four and six in two more. Each time the agent resolved them by hand with the same rule (for each locator keep the side that re-pinned since the merge base, or take the other side and re-pin against the merged file when both did; keep both sides' revision rows), and a reviewer then had to check by hand that no row was lost. The tool already holds everything that rule needs. Separately, `urzua fix --conflicts --apply` with no ids answers only "pass --ids <record,...> or --force", so the agent has to find the conflicted records itself though the command knows them.

## Done means

- For a conflicted record whose pinned file has no conflict markers in the working tree, `fix --conflicts --apply` keeps both sides' revision rows in date order and, per locator, takes the side that re-pinned since the merge base; where both sides re-pinned, it re-pins against the merged file and writes one row saying both sides had re-pinned and the pin was recomputed from the merged content, with the attestation the command was given; shown by a test for each of the three cases.
- It still refuses when the pinned file itself has conflict markers, naming the file to resolve first.
- No revision row from either side is lost and no Done-means text changes, shown by a test that compares the resolved record with both parents.
- Run with `--apply` and neither `--ids` nor `--force`, the answer lists the conflicted records and carries a `next_action` that names them.
- The acceptance corpus holds a case built from a real three-way merge of two branches that both re-pinned one file.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-18 | Initial record, `Status: Planned`. **Why:** on 2026-11-17 agents resolved 34 conflicted records by hand across four merges, by one rule the tool could apply. | **structural** |
