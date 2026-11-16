---
Stable-Id: 01M7HVZZ7RE9TDFS6WV4BMHSW7
Status: Planned
Phase: '1'
Track: embodiment-model
Blocked-on: null
---
# 288 — A pin that changes without a re-pin row is a finding

## What

Report a record whose evidence pin changed between the base and the head when no revision-log row written by the re-pin tooling accompanies the change.

## Why

Found on 2026-11-16 while urzua's own builders and reviewers used the tool to land a week of pull requests. Three reviewers reproduced the same gap: changing a pin's hash by hand to the file's current hash turns `check` green, under `--base` too, with no finding and no reason. The rule that a re-pin needs a reason holds only for the command, not for the record.

## Done means

- `urzua check --base` reports a record whose pin hash differs from the base without an accompanying re-pin row, naming the locator, shown by a test with a hand-edited pin.
- A re-pin made with `urzua fix --rebaseline --apply`, a renewal and a pin recomputed by `urzua fix --conflicts` produce no such finding.
- A pin added to a locator that had none is treated as the tool treats a first pin today, and that is stated in the spec.
- The capability is exercised in urzua's own `.urzua/config.yaml` and corpus, and `urzua check` reports 0 unwaived findings.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from friction met while using urzua on its own repository on 2026-11-16; the maintainer said that day that issues met in using and adopting the tool are improvement points. | **structural** |
