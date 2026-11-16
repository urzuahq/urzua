---
Stable-Id: 01M7GX5MEZDHKE1NH9CZGNSE8D
Status: Proposed
Date: 2026-11-16
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
---
# 102 — A person accepts a record in chat: the harness asks, records the answer and allows that one edit

## Context

`ADR-89` gives acceptance to people: an agent drafts a record and only a maintainer moves it to a status the `authority` table
reserves. The harness enforces this locally through the `acceptance` guardrail, which refuses the agent's edit, and the
instructions block it installs tells the agent not to accept or reject a record itself.

In daily use this produces a dead end. The maintainer reads a Proposed ADR in the chat, says it is accepted, and the agent
answers that it has no permission and that the maintainer must edit the file by hand outside the agent. The person who holds
the authority is present, has decided, and still has to leave the conversation to type one line. The maintainer reported this
on 2026-11-16 as happening on every ADR, and asked for the agent to ask for approval in the chat with options to click.

The guardrail cannot tell a maintainer's instruction from an agent's own initiative, because the hook sees the edit and not
the conversation. What it can see, on an agent that supports it, is a structured question the agent put to the person and the
answer the person chose, or it can raise a confirmation prompt itself.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Keep deny only: a person edits outside the agent | Nothing to build; the strongest local guarantee | The dead end stays; people turn the guardrail off to get work done, which is weaker than any option below |
| B. A config switch that lets the agent make the move whenever asked in prose | Simple | The hook cannot verify that anyone asked; an agent's own initiative passes; this is the guardrail switched off |
| C. The harness refuses, tells the agent to ask the person a structured question, records the person's answer as a one-use approval, and allows only the edit that matches it | The person decides in the chat with one click; the agent cannot write the approval; each acceptance leaves a receipt | Depends on what the agent host exposes to hooks; the proof is local, so CI can check the receipt exists but not who clicked |
| D. The hook raises the host's own confirmation prompt on the edit | Works without a structured question tool; the agent cannot answer it | Less context in the prompt; behaviour differs between the host's permission modes and is undefined in some |

## Decision

*In the context of a maintainer who is present in the conversation and has decided, facing a guardrail that can only refuse
and send them to a manual edit, we decided that the `acceptance` guardrail has a second, opt-in mode in which the harness
obtains the person's answer in the chat and allows the one edit that answer covers, to achieve acceptance that stays a
person's act without leaving the conversation, accepting that the evidence is local to the session and weaker than a
host-verified identity.*

1. **Configuration.** The `acceptance` guardrail takes a mode. The default is today's behaviour, refuse; a configuration that
   does not declare the mode behaves as before (`ADR-101`, standing principles). The new mode is asked for by name.
2. **Refusal that says how to proceed.** With the mode on, an agent's move of a record to a reserved status with no approval
   on file is still refused, and the refusal tells the agent to put a structured question to the person naming the record,
   its title and the move, with the choices accept, request changes and reject, and then to retry. For a call made by a
   subagent, which cannot question the person, the refusal says to hand back to the main session.
3. **The answer becomes a one-use approval.** A hook on the question tool reads the answer the person chose from the tool's
   result, never from anything the agent supplied, and on accept writes an approval where the agent cannot write. The
   approval names the record, the target status, a hash of the record body the person was shown, and the session, and it
   expires after a short configured time.
4. **The edit consumes it.** The next move of that record to that status, in that session, with a body matching the hash, is
   allowed and the approval is deleted. A changed body, another record, another status or an expired approval is refused.
5. **The harness writes the receipt.** The acceptance fields and a revision-log row naming the person, the method (chat) and
   the time are written by the harness, not by the agent.
6. **Fallback.** Where the host gives no trustworthy answer from a structured question, the hook raises the host's own
   confirmation prompt on the edit (option D), and where the host's permission mode does not guarantee that prompt is shown
   to a person, the edit is refused.
7. **Checkable afterwards.** A rule requires a record at a reserved status to carry an acceptance receipt, so a status
   change made from a shell with no receipt is a finding at commit and in CI. `diff.decider-required` is unchanged: CI still
   judges who made the pull request.
8. **Instructions.** The installed instructions block tells the agent to ask the person with a structured question when a
   record is to be accepted or rejected, and never to change a status without that answer.

The person named in a receipt comes from the configured identity for the session, as for other attested writes; a chat answer
does not prove which person clicked. A repository that needs that proof keeps acceptance in a pull request, where the host
names the actor.

## Reversibility

Easy to walk back: the mode is opt-in, and removing it returns every repository to refuse. The cost of getting it wrong is an
acceptance recorded without a person's decision, which is the failure the guardrail exists to stop; that is why the answer is
read only from the tool's result, the approval is single-use and bound to the body shown, and an uncertain host mode refuses.
Receipts already written stay valid records of what happened.

## Consequences

- A maintainer accepts or rejects a record with one click in the conversation, and the agent stops answering that it cannot.
- The local guarantee in this mode rests on the agent host: that a hook sees the person's real answer and that the agent
  cannot supply one. Each adapter states which of options C and D it supports, and one that supports neither offers refuse
  only. This must be verified on each host before the mode is offered there.
- Acceptance gains an audit trail it did not have: every accepted record names how it was accepted.
- A new place the agent must not write (the approvals) joins the harness's protected state.
- `ADR-89` needs an amendment stating that an approval obtained this way is a person's act, and what it does and does not
  prove.
- Rejection and requested changes are answers too: the agent gets them as the result of its question and no approval is
  written.

## References

`ADR-89` (the harness and the authority model), `ADR-78` (the harness's enforcement decisions and the override), `ADR-101`
(people own decisions; opt-in configuration), `SPEC-24` (the adapter contract).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Drafted as Proposed. **Why:** The maintainer reported on 2026-11-16 that every acceptance ends with the agent saying it has no permission and asked for approval by a question in the chat. | **substantive** |
