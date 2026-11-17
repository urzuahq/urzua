---
Stable-Id: 01M7KN9RDHSDPVS80PTSTRR984
Status: Planned
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 329 — urzua explain prints what a repository's configuration declares

## What

Add a read-only form of `urzua explain` that prints, from the configuration alone, what this repository declares for its record types, short enough for an agent to read at the start of a session.

## Why

On release 0.4.7 an agent new to a repository cannot learn its record model from the tool. `urzua explain` takes a path or a rule id; `urzua rules` reads no configuration; `urzua doctor` reports health; the ordering command assumes the types are already understood. The only way to learn which fields a type carries, which values a field may hold, what a reference may point at, which states exist and how a record moves between them is to read the configuration file (336 lines in the reporting repository) and whatever notes the repository wrote for itself. The configuration is the source of truth and the tool already parses all of it; an agent that cannot ask the tool writes its own summary, which then drifts. The maintainer looked for this under `urzua explain` first.

## Done means

- `urzua explain --types` prints every declared record type and `urzua explain --type <name>` one of them, as JSON, read from the configuration and nothing else; the name was accepted by the maintainer on 2026-11-17: "types is more generic and more agent friendly and machine friendly" than a name that calls it a workflow.
- For each type the report gives: its identifier prefix and directories; required and optional fields; each field's declared values; each reference field with the types it may name; its states with the initial ones and the allowed moves; each declared requirement and prohibition with its conditions as one sentence; any gate on its states or paths with who may act; the template in use; and the exact command that creates a record of the type, shown by a test against a configuration that uses each of these.
- The report lists the rules this configuration sets, with their levels, and separately the rules that are on because of a declaration, each naming the declaring key.
- Nothing in the report is a built-in default the configuration did not change, except where a default decides behaviour a reader needs (the initial state, a rule on by declaration), and those are marked as defaults.
- Every entry carries the configuration key path and line it was read from, so a reader can go to the declaration.
- The report for a repository with three types of ordinary size stays under a size stated in the test, and the agent harness's session-start report names the command.
- The report states intent nowhere: it describes what is declared, not why.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Planned`. **Why:** Reported on 2026-11-17 by an agent in an adopting repository at the maintainer's prompting; the maintainer had asked "if another agent comes along, can they figure out what's the workflow quickly through the urzua tool?", expected `urzua explain` to do it, and said a repository should not need its own working document for this. | **structural** |
