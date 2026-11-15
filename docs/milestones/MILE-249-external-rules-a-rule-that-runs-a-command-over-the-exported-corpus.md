---
Stable-Id: 01M7FW08P65PK369Z9QG3STYVK
Status: Planned
Phase: '2'
Track: engine-extensibility
Blocked-on: null
---
# 249 — External rules: a rule that runs a command over the exported corpus

## What

Let a configuration declare a rule that runs a command, passes it the exported corpus as JSON, and reads findings back in the report's own shape, under a reserved prefix.

## Why

the audit of 2026-11-15's extensibility experiments found a closed rule catalog: a team cannot add its own check without forking, and the only route today is a separate CI script over `export`.

## Done means

- A decision record states the contract, the reserved prefix, and what a command may and may not do (time limit, no network by default, failure is reported, never a pass).
- A fixture repository's external rule produces a finding that appears in `check` with severity, file and message, and takes part in `fail_on` and waivers.
- A command that fails or times out makes the run report the rule as not run.
- The rule table and `urzua rules` list external rules as such.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
