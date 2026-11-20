---
Stable-Id: 01M7VN7TF0WB5843KAZ8WXY7TG
Status: Proposed
Date: 2026-11-20
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
---
# 105 — Where the host shows no prompt, the person approves a gated move by answering urzua's question

## Context

`ADR-102` lets a person approve an agent's gated move (an ADR to `Accepted`, say) inside the session. As amended on
2026-11-17 and 2026-11-18 the route is the host's own permission prompt on the edit, and every permission mode in which
Claude Code shows no prompt (`bypassPermissions`, `acceptEdits`, `auto`, `dontAsk`, `plan`) refuses the move, leaving a
hand edit outside the session as the only way through.

The maintainer runs every session in `bypassPermissions`, so in-session approval never works for him. His direction on
2026-11-20: "we should support both ... the current way is not working for any of the repos I work on" — keep the
permission prompt and add a route through Claude Code's question tool, AskUserQuestion.

The amendment of 2026-11-16 to `ADR-102` allowed a chat question only under limits: urzua writes the wording and it
carries a value the agent cannot guess; only an offered choice counts, never free text, a timeout or a dismissal; and
"the mode is refused in a session where another hook or a host program can answer questions or permission prompts".

What was observed on 2026-11-20, in interactive sessions of Claude Code 2.1.323 in `bypassPermissions` with logging
hooks on AskUserQuestion:

1. The agent put an `answers` field in the tool's input. The question was still shown; the person chose Decline and the
   tool returned Decline. The hook before the call saw the agent's `answers`; the hook after it saw `tool_input`
   rewritten to the collected answers, equal to `tool_response`. A prefill is therefore visible only before the call.
2. An idle timeout (`askUserQuestionTimeout`) returned `answers: {}` with `afkTimeoutMs`; a highlighted option was not
   submitted.
3. A PreToolUse hook that answered through `updatedInput` showed the person nothing, and the hook after the call saw an
   answer shaped like a person's.

Claude Code's hook reference says of `answers`: "Claude doesn't set this field; supply it via `updatedInput` to answer
programmatically" (code.claude.com/docs/en/hooks), and that a `-p` run offers the tool only with a permission host.

The maintainer's own sessions run under an Agent SDK host (`CLAUDE_CODE_ENTRYPOINT=t3-code`), which answers
AskUserQuestion through its own interface, and his user settings register a `PermissionRequest` hook matching every
tool. The limit of 2026-11-16 refuses exactly these sessions, so read literally it would make the route useless where
it is needed. The same host and the same hooks can already answer the permission prompt that `ADR-102` accepts today.

The maintainer's framing, 2026-11-20: "we don't pretend to say that we can, we are some sort of like secure sandbox
harness. Like, there are ways around it. There's always a way around it. And that's not what we're trying to do. We're
just putting in guards for the normal cases ... because we're offloading the trust, we aren't kind of trying to be the
trust anchor."

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Keep refusing where no prompt is shown | Nothing to build | The maintainer's sessions never get in-session approval; the dead end `ADR-102` set out to remove stays |
| B. Question route, refused wherever another hook or a host program could answer (the 2026-11-16 limit) | Matches the earlier limit | Refuses the maintainer's own host and settings; the hook cannot see every such party anyway, so the refusal is partial |
| C. Question route trusting the host and its permission layer as the permission prompt already does, each receipt naming the route and the host | Works in bypass mode and under an SDK host; no new answering party beyond what the prompt route trusts; cheap checks stop the normal-case mistakes | A hook or host program that answers in the person's place gets through, as it can for the prompt |

## Decision

*In the context of a maintainer who works in a permission mode that shows no prompt, facing an approval route that then
always refuses, we decided that under `acceptance.in_session: ask` a gated move the host will not prompt for is
approved by the person's answer to one question urzua writes, trusting the host and its permission layer to report that
answer as the permission prompt already does, to achieve in-session approval as a guard for the normal case, accepting
that a hook, a host program or a shell command that answers or writes in the person's place gets past it.*

The maintainer chose this on 2026-11-20, asked through AskUserQuestion, by picking "Trust them (Recommended)", whose
description read: "Same trust as the permission prompt: the host or a permission hook can answer. Each receipt row
records which route answered, and the host's name where the hook can see it. Works in T3 Code."

1. **Configuration.** The route is part of `in_session: ask`; there is no new value. Where the host shows a prompt
   (Claude Code's Manual mode) the prompt is still used, as `ADR-102` describes.
2. **The question.** Where no prompt is shown, the refusal of the gated move gives the agent the exact input of one
   AskUserQuestion call. Its question begins `urzua approval <nonce>:` and names the record, the field, the status
   left and entered, the gate and the decider group, all read from the record and the configuration. The choices are
   exactly `Approve <nonce>` and `Decline`. Unlike the host's prompt, which shows the person the whole file, the question
   does not show the content, so it says what else the approval covers: how many other lines of the record the edit
   changes besides the status line (each approved with it), the date fields it fills, and the receipt row it adds. A
   person who wants to see the change reads the agent's proposed edit, or uses the Manual mode, where the prompt shows it.
   The challenge is stored with the record's path, the two statuses, a hash of
   the content the agent proposed, the session and the time.
3. **Before the call.** urzua's hook on the tool refuses an urzua question whose input already carries `answers` and
   withdraws its challenge; refuses one whose wording or choices differ from what was issued, that shares its call with
   other questions, that comes from a subagent or another session, or whose challenge expired; otherwise marks it as
   asked by that call.
4. **After the call.** urzua's hook reads the answer from the tool's result, never its input. Only the exact approve
   label, for a question marked asked by the same call in the same session, approves; a decline, free text, an empty
   answer or an idle timeout approves nothing and withdraws the challenge. An approval records the host the hook ran
   under, as `CLAUDE_CODE_ENTRYPOINT` names it.
5. **The edit uses it once.** The next edit making the same move, of the same record, with the same proposed content, in
   the same session, within five minutes, takes the approval (by an atomic rename, so two edits cannot both use it) and
   is written with the receipt row of `ADR-102`, which names the question, the person as the session reports them and
   the host. A challenge must be answered within ten minutes. Anything else is refused again with a new question.
6. **Where it is kept.** Challenges and approvals live in `<git common dir>/urzua/approvals`, which the edit tools
   cannot write.

This narrows the limit of the amendment of 2026-11-16 to `ADR-102` that a session where another hook or a host program
can answer is refused: such parties are trusted as for the permission prompt, and the receipt records the route. The
other limits of that amendment are kept: urzua writes the wording and the question carries a value the agent did not
choose. The rule for the answer is one rule: the answer the host reports must equal the offered label `Approve <nonce>`
exactly. Choosing that option meets it; free text the person types that repeats the label exactly, nonce included,
cannot be told apart from choosing it and meets it too; any other text, an empty answer or a timeout does not.

## Reversibility

Easy. The route only runs under `in_session: ask` where no prompt is shown, and only once the hooks on the question
tool are installed; removing them or the mode returns those sessions to refusal. Receipt rows already written stay true
records of what the host reported.

## Consequences

- A person working in `bypassPermissions`, or under an Agent SDK host, approves a gated move from the session.
- Known ways past it, listed rather than closed: another PreToolUse or `PermissionRequest` hook, or a host program, that
  answers the question in the person's place; a shell command that writes an approval file into the git directory
  (the documented accepted risk, which no check reports); an agent that runs `urzua harness run answer` itself from a
  shell; a person who approves without reading. The receipt row is text and stays unverified, as under `ADR-102`.
- `urzua harness install` registers two more hooks for Claude Code (before and after AskUserQuestion); a repository
  that has not reinstalled gets the question but records no answer, so the move stays refused.
- The installed instructions block says that a gated edit may be refused with a question to ask exactly as given.

## References

`ADR-102` (in-session approval; this narrows its amendment of 2026-11-16), `ADR-89` (the harness and the authority
model), `SPEC-24` (the adapter contract), `SPEC-3` (configuration), `MILE-353` (the build).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-20 | Drafted as Proposed. **Why:** the maintainer said on 2026-11-20 "we should support both ... the current way is not working for any of the repos I work on", and chose "Trust them (Recommended)" for the trust question. | **substantive** |
> | 2026-11-20 | Revised while Proposed: the question says how many other lines the edit changes and which date fields it fills, and the difference from the prompt route is stated; the answer rule is stated as one rule. **Why:** an independent review of #612 found that the person never sees the content in the question route and that the answer rule read as two rules. | **substantive** |
