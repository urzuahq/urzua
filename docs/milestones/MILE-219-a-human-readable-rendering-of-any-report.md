---
Stable-Id: 01M7FVRGR4Y1DMDV15PB98SV6R
Status: WontDo
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 219 — A human-readable rendering of any report

## What

Let a person read a report at a terminal without `jq`: a renderer that turns the JSON report of any command into a short text summary (status, findings grouped by file, the next action), without changing what stdout carries by default.

## Why

the audit of 2026-11-15 found that output is JSON only, by `ADR-23` and `ADR-26`, and the only human rendering lives in the gate action. A new user running `urzua check` locally sees a wall of JSON, and usage errors arrive as an escaped string inside JSON.

## Done means

- A decision record states how the rendering is offered (a flag, a subcommand that reads a report, or detection of a terminal) and how it stays consistent with `ADR-23` and `ADR-26`.
- The rendering of a `check` report with findings names each file, rule, severity and message, and ends with the suggested next command.
- The rendering is produced from the report JSON alone, shown by a test that renders a saved report.
- The gate action's comment and the local rendering come from one implementation.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
> | 2026-11-16 | Decided: this milestone will not be done. **Why:** The maintainer decided on 2026-11-16: "we just always output JSON. Only agents use this software." Output stays JSON only (`ADR-23`, `ADR-26`). The Status moves to WontDo in a follow-up, because the first WontDo milestone retires `WAIVER-0052` and the places that cite it. | **substantive** |
> | 2026-11-16 | `Status` moved from `Planned` to `WontDo`, recording the decision of the row above. **Why:** the maintainer decided on 2026-11-16: "we just always output JSON. Only agents use this software." This is the first `WontDo` milestone, so the `milestone` type's `WontDo` exemption is exercised and the waiver that excused it matching no record is retired, with `SPEC-10` (0.24) updated. | **structural** |
