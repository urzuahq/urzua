---
Stable-Id: 01M6NX3687XF81EP8GD4JSYSEQ
Status: Planned
Phase: '0'
Track: governance-process
Implements: ADR-89
Depends-on: MILE-198, MILE-200
Blocked-on: null
---
# 199 — Generate a drafting command for every record type and add the lifecycle commands amend, repin and done

## What

Generate one drafting command for every record type the configuration declares from the single `ADR-70` template, and add the lifecycle commands `/amend`, `/repin` and `/done`, installed by `urzua harness install`. The type name and description come from `.urzua/config.yaml`; no command carries schema knowledge.

## Why

`.claude/commands/` holds `adr`, `rfc` and `milestone` for seven record types. There is no command for a bug (the most common record an agent files), a waiver, a condition, a dated amendment to an Accepted ADR, a re-pin or marking work done, so an agent hand-writes those with the generic tools and no guard (`ADR-89`, Context). A bug cannot be filed `Fixed` through the command path because `scripts/draft-record.sh:55-57` refuses any changed scaffold value, so a fixed-in-PR bug is hand-written; `/done` is the sanctioned route, and it stands on the 2026-11-05 amendment to `ADR-70`, which lets a lifecycle command set a status only when the engine computes it from evidence. Two re-pin reasons on 2026-11-05 did not describe their change, and a command that makes the reason a required, shown input is the cheapest guard.

## Done means

- After `urzua harness install`, a command exists for every record type in this repository's configuration (adr, rfc, milestone, bug, condition, waiver, spec), generated from the configuration; adding a type to the configuration and running `urzua harness update` adds its command, and a test plants that.
- `/amend` appends a dated `## Amendment` section and a revision row to an Accepted ADR and leaves its Decision text unchanged; `/repin` runs `urzua fix --rebaseline` only with a non-empty `--reason` and prints the re-pin and its reason in the form a PR body lists; `/done` records the `Realized-by` evidence and then runs `urzua fix --apply`, so the delivered status is the engine's computed one and the command never writes a status itself, for a type whose status the configuration derives from evidence; it files a bug as `Fixed` in the PR that fixes it the same way, and refuses a status an acceptance gate names. Each has a planted pass and a planted refusal that fail before the change.
- No installed command contains a record-type name, field name or status value that is not read from the configuration (a test scans the generated files for the literals).
- This repository uses them: its bundle holds the generated commands, and at least one record in the PR that merges this milestone is filed through `/bug`, `/condition`, `/waiver`, `/amend` or `/done`, with `urzua check` reporting it clean and its examined count above zero.
- The `ADR-70` amendment of 2026-11-05 (lifecycle commands may set a status only when the engine computes it from evidence) is accepted before this milestone merges; until then `/done` is held. `/repin` and `/amend` need no change to `ADR-70`, and `/done` validates against the status graph of `ADR-72`.
- `SPEC-19` documents the command set with a revision row.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** `ADR-89` (Proposed) decides the installable agent harness bundle on maintainer decisions of 2026-11-05 and no milestone builds this part; blocked on the maintainer accepting it. | **substantive** |
> | 2026-11-05 | `Depends-on: MILE-200` added, since `/done` refuses a status an acceptance gate names; `/done` now runs `urzua fix --apply` and writes no status itself, resting on the `ADR-70` amendment of 2026-11-05 (proposed, pending maintainer acceptance); the reference to an open question that did not exist is removed. **Why:** Independent review of pull request 355 and the maintainer decision of 2026-11-05 on `/done`. Still `Planned`. | **substantive** |
