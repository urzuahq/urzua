---
Stable-Id: 01M7GX85JSRQ2MWPC8A20JD9GN
Status: Planned
Phase: '1'
Track: accountability-identity
Blocked-on: null
---
# 273 — Build acceptance in chat: the harness asks the person, records the answer and allows that one edit

## What

Build the opt-in mode of the `acceptance` guardrail that `ADR-102` proposes: the refusal tells the agent to ask the person a structured question, a hook turns the person's answer into a one-use approval the agent cannot write, the matching edit is allowed, and the harness writes the acceptance receipt.

## Why

The maintainer reported on 2026-11-16 that every acceptance ends with the agent saying it has no permission and sending them to edit the file by hand, and asked to approve from a question in the chat. Blocked until `ADR-102` is accepted.

## Done means

- `ADR-102` is accepted, and `ADR-89` carries an amendment stating what an approval given in chat proves and does not prove.
- On the Claude Code adapter a recorded test shows what a hook receives as the person's answer to a structured question, whether the agent can supply that answer itself, and whether a hook's confirmation prompt is shown in each permission mode; the adapter offers only what those results support.
- Where the host supports a tool that requires the person's interaction on every call, an urzua tool asks for the acceptance with a prompt urzua wrote, and on approval makes the status change and writes the receipt itself (`ADR-102`, amendment of 2026-11-16); the chat question is offered only where the host has no such tool.
- With the mode on, an agent's move to a reserved status with no approval is refused with the instruction to ask, and after the person chooses accept the same edit is allowed once, shown by a test.
- An approval is refused for another record, another status, a changed body, another session or after its expiry, shown by a test per case.
- The agent cannot write an approval: an edit or shell write to where approvals are kept is refused or reported like any protected harness state.
- The harness writes the acceptance fields and a revision-log row naming the method, and a rule reports a record at a reserved status with no receipt.
- When an agent's edit makes a gated move, the harness returns the host's ask-the-person decision for that edit; the prompt names the record, the status it leaves and the status it enters, and the gate; approving writes the edit and declining refuses it. No command is added for it: the edit already states the record, the field and the move, and the gate table already says which moves are reserved, shown by a test of approve and of decline.
- A person can also approve by typing a harness command that names the record. A hook on the person's own input sees it, which an agent cannot produce, and grants a one-use approval for that record's pending gated move, whatever status the gate names. The command belongs to the harness, not to the engine's command line, and is named for approving a gated move, not for any one status, shown by a test that the same text written by the agent grants nothing.
- An approval given in an agent session is recorded as that ("approved in an agent session by" the person), never as the person having edited the file, and the local hook and the CI rule both honour the same record of it, shown by a test of each.
- A configuration that does not declare the mode behaves as before, shown by a test.
- The installed instructions block tells the agent to ask the person, and this repository's own configuration uses the mode.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** The maintainer asked for acceptance by a question in the chat on 2026-11-16; `ADR-102` is drafted as Proposed and this tracks the build. | **structural** |
> | 2026-11-16 | Added a Done-means item for the approval tool as the first mechanism. **Why:** `ADR-102`'s amendment of 2026-11-16, accepted by the maintainer, makes it the primary route and the chat question the fallback. | **substantive** |
> | 2026-11-17 | Added two Done-means items: the host's ask-the-person decision as the way a gated edit is approved, and one record of an approval that the local hook and CI both honour. **Why:** an adopting repository reported on 2026-11-17 that its maintainer approved a record in chat and expected that to be enough ("I want to be able to accept decisions if I give you my explicit approval through the chat"), that the agent's change was then stopped at commit by the rule that checks who made a gated move, and that the same would have stopped the person; The adopting repository's agent relayed the maintainer's choice the same day: the host asks instead of refusing, and no accept command, because a command named for one status of one profile "adds weird behaviour to the engine" where types, statuses and gates are configuration. Recorded by the orchestrator on 2026-11-17 from that relay, pending the maintainer's confirmation and an amendment to `ADR-102`, which puts an urzua tool first. | **substantive** |
> | 2026-11-17 | Added a Done-means item: a typed harness command as a second way to approve. **Why:** the maintainer said on 2026-11-17: "I also think the typed command would be useful, actually having other typed commands in the harness is actually good to have. They should be a good feature to have in the harness work that we're doing." His earlier objection, relayed the same day, was to an engine command named for one status; a harness command named for approving a gated move is the orchestrator's reading of how the two fit, pending his review. | **substantive** |
