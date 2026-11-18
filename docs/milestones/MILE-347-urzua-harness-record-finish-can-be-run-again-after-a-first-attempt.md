---
Stable-Id: 01M7QDN89ZR460DVWDJHDMAQ73
Status: Planned
Phase: '1'
Track: write-path
Blocked-on:
---
# 347 — urzua harness record finish can be run again after a first attempt

## What

`urzua harness record finish <record>` can be run again on a record whose earlier finish reached `check` in the same working tree, so that a record corrected after `check` reported findings goes through the same refusal, staging, index and check steps as the first time.

## Why

The drafting commands tell the agent to fix the record and run the finish step again when `check` reports findings. A second run does not do that. On 2026-11-18, `urzua harness record finish` on a freshly scaffolded milestone succeeded, and the same command run again on the same file answered `was not scaffolded by 'urzua harness record new'` and stopped. The cause is in `rust/crates/urzua-cli/src/harness/record.rs`: the finish step deletes its snapshot of the scaffold once `check` has run, whether or not `check` reported findings, and a missing snapshot is what the refusal reports. A finish refused before `check` keeps the snapshot and can already be run again. After a first attempt the agent therefore has to stage the record, regenerate the indexes and run `check` by hand, without the step that refuses a changed scaffold value, which is the protection the command exists for.

## Done means

- Running `urzua harness record finish` twice on the same scaffolded record succeeds both times when nothing the scaffold decided was changed; shown by a test in `drafting_commands_integration`.
- A second run after an edit that changes a scaffold-decided value (state, date, id) between the runs is refused with the same message as on a first run, which means what the scaffold decided is still known after `check` has run; shown by a test.
- A second run after the first reported `check` findings and the record was corrected regenerates and stages the indexes and reports the examined count; shown by a test.
- A file that `urzua harness record new` never scaffolded is still refused, and the message says so; shown by a test.
- The generated drafting commands' step that says to run the finish step again is true as written, shown by a test in `drafting_commands_integration` that follows the steps of a generated command (for example `.claude/commands/milestone.md`) through a first finish with findings, a correction and a second finish.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-18 | Initial record, `Status: Planned`. **Why:** a second `urzua harness record finish` on the same record was refused as not scaffolded on 2026-11-18, though the drafting commands say to run it again. | **structural** |
