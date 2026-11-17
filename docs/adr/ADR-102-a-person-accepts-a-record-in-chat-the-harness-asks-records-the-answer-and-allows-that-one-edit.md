---
Stable-Id: 01M7GX5MEZDHKE1NH9CZGNSE8D
Status: Accepted
Accepted-on: 2026-11-16
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

## Amendment (2026-11-16): the approval is asked for by an urzua tool first, and by a chat question as the fallback

Accepted by the maintainer on 2026-11-16. The Decision is unchanged: acceptance in the conversation stays an opt-in mode, the
approval is single-use and bound to what the person was shown, and the harness writes the receipt. This changes which
mechanism is tried first, after a review on 2026-11-16 of what agent hosts document.

The review found two weaknesses in asking through the agent's own question tool, which points 2 and 3 of the Decision make
the primary route. The agent writes the question's wording, so the hook sees which choice was picked but cannot know what
text the person read, and "a hash of the record body the person was shown" cannot be established from it. And on the host
reviewed, the answer to such a question can be supplied by something other than a person: another hook, a host program, or
an idle timeout, and a free-text reply exists beside the offered choices.

The order of mechanisms is therefore:

1. **An urzua tool that asks.** Where the host lets a tool require the person's interaction on every call, in a way no hook
   and no permission mode can skip, the agent calls an urzua tool naming the record and the move. urzua reads the record,
   writes the prompt itself (the record's id, title, the move and what it means), the host shows it to the person, and on
   approval urzua makes the status change and writes the receipt itself. No approval is stored between the answer and the
   edit, so there is nothing for an agent to forge or replay.
2. **A chat question, as the fallback.** Where the host has no such tool, points 2 to 5 of the Decision apply, with these
   limits: only a question whose wording urzua generated, carrying a value the agent cannot guess, counts; only one of the
   offered choices counts, never free text, a timeout or a dismissal; and the mode is refused in a session where another
   hook or a host program can answer questions or permission prompts.
3. **The host's confirmation prompt**, point 6 of the Decision, remains the last fallback, and an uncertain permission mode
   still refuses.

An adapter states which of the three it supports, each verified on the host by a test and not taken from its
documentation. The refusal of point 2 of the Decision names the mechanism the adapter supports.

## Amendment (2026-11-17): the host asks the person about the edit itself, a typed harness command is the second way, and there is no accept command

On 2026-11-17 the maintainer was asked to confirm that the host asks about the edit, with no accept command, and that this
record be amended to say so. He answered: "I'm happy for you to amend ADR 102. I'll give you my acceptance." Earlier the
same day he said of a typed command: "I also think the typed command would be useful, actually having other typed commands
in the harness is actually good to have." And an adopting repository's agent relayed his objection to a command that
accepts a record: it "adds weird behaviour to the engine", because types, statuses and gates are configuration and such a
command names one status of one profile. Those three things are the maintainer's. Everything marked "specified" below is
decided by the orchestrator on 2026-11-17, pending maintainer review.

What prompted it. An adopting repository's maintainer approved a decision record in chat and expected that to be enough;
the agent's change was then stopped at commit, and the same rule would have stopped the person.

The Decision is unchanged: acceptance in the conversation stays an opt-in mode, an approval is single-use and bound to what
the person was shown, the agent cannot write one, and the harness writes the receipt. This changes the order of mechanisms
set by the amendment of 2026-11-16 and rules one mechanism out.

1. **The host asks about the edit.** When an agent's edit makes a gated move, the harness returns the host's
   ask-the-person decision for that edit. Approving writes the edit; declining refuses it. This is now the first mechanism,
   where the amendment of 2026-11-16 made it the last.
   Specified: the hook writes the prompt's reason string, never the agent. It names the record, the status it leaves, the
   status it enters and the gate, read from the record and the configuration, and the person sees the host's own display of
   the proposed edit. That answers the concern of 2026-11-16 that the agent writes the question. The edit is refused in any
   permission mode the adapter lists as approving edits without asking; each listed mode has a test that the gated edit is
   refused, and a mode the adapter does not list is treated as one that approves without asking.
2. **A typed harness command.** The person may instead type a harness command that names the record. A hook on the
   person's own input sees it, which the agent cannot produce.
   Specified: it grants a one-use approval bound as points 3 and 4 of the Decision bind one (the record, the status it
   enters, a hash of the record body, the session and a short expiry). The command gives no choice of status: the approval
   is for the pending move the harness shows the person in its reply. The command belongs to the harness, is named for
   approving a gated move and not for any one status, and the same text written by the agent grants nothing.
3. **No accept command in the engine.** The edit already states the record, the field and the move, and the gate table
   already says which moves are reserved, so no new verb is needed.

Specified: effect on the Decision's points. Point 1 (the mode) and point 7 (the rule that requires a receipt) are kept.
Point 2 is narrowed: the refusal names the mechanism the adapter supports, which is now the host's prompt on the edit or
the typed command, not a structured question. Point 3 is superseded for the first mechanism, where nothing is stored, and
kept for the typed command. Point 4 is kept for the typed command and does not apply to the host's prompt, whose approval
is the edit itself. Point 5 is kept: the harness writes the receipt on either path. Point 6 becomes the first mechanism.
Point 8 is narrowed: the instructions block tells the agent that its gated edit will prompt the person and that it must
never change a status without that answer.

Specified: what is written. Approving the host's prompt stores no approval; the edit is allowed or refused. The harness
writes the receipt in the same edit (the acceptance fields and a revision-log row reading that the move was approved in an
agent session by the named person), and that receipt is the one record both the local git hook and the rule that checks in
CI who made a gated move read. The typed command stores a one-use approval that the next matching edit consumes, and the
same receipt is written then. A receipt never states that the person edited the file.

Specified: the urzua tool that asks, and the chat question, of the amendment of 2026-11-16 are no longer the first and
second mechanisms. An adapter may still offer the tool where its host supports one; it is not required.

Limits. A session not rooted in the repository loads no hook, so it gets neither a refusal nor a prompt; that is outside
this mechanism. A person who approves prompts without reading them defeats the prompt, so the prompt is raised only for
gated moves and its reason states the move. An approval proves who approved only as far as the host can tell who is at the
keyboard, and CI still sees a pull request as authored by whoever's credentials pushed it.

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
> | 2026-11-16 | Status changed from Proposed to Accepted. **Why:** Accepted by the maintainer on 2026-11-16. | **substantive** |
> | 2026-11-16 | Amendment added: an urzua tool that requires the person's interaction is the first mechanism and the chat question the fallback. **Why:** Accepted by the maintainer on 2026-11-16, after a review of agent hosts found that the agent writes a chat question's wording and that something other than a person can answer it. | **substantive** |
> | 2026-11-17 | Amendment added: the host asks the person about the edit itself as the first mechanism, a typed harness command as the second, and no accept command in the engine. **Why:** Asked on 2026-11-17 to confirm that the host asks about the edit, with no accept command, and that this record be amended, the maintainer answered "I'm happy for you to amend ADR 102. I'll give you my acceptance."; the amendment marks what he said, what was relayed, and what the orchestrator specified pending his review. An adopting repository had met the refusal on a record its maintainer had approved in chat. Recorded by an agent through the shell at the maintainer's instruction. | **substantive** |
